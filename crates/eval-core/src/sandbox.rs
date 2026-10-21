//! Process isolation and bounded command execution.
//!
//! On Linux the default backend is bubblewrap (`bwrap`). It creates a
//! user/mount/pid/network namespace, mounts the host filesystem read-only,
//! bind-mounts only the workspace read-write, and drops the host environment.
//! If `bwrap` is not available, execution is refused unless the operator
//! explicitly opts into unsandboxed mode with
//! `AGENT_BENCH_ALLOW_UNSANDBOXED=1`.
//!
//! This module also provides a watchdog that kills the whole process group on
//! timeout, drains stdout/stderr with a size cap, and never lets a background
//! process keep the evaluator blocked on a pipe forever.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Sandbox {
    bwrap: Option<PathBuf>,
    allow_unsandboxed: bool,
    allow_network: bool,
    default_max_output_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct ExecRequest {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    /// (source, destination) pairs bind-mounted read-only inside the sandbox.
    /// Destination must be inside `cwd`.
    pub extra_ro_binds: Vec<(PathBuf, PathBuf)>,
    pub extra_env: Vec<(String, String)>,
    pub max_output_bytes: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecOutput {
    pub status_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub sandboxed: bool,
}

impl ExecOutput {
    pub fn combined(&self) -> String {
        let mut s = String::new();
        if !self.stdout.is_empty() {
            s.push_str(self.stdout.trim_end());
        }
        if !self.stderr.is_empty() {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str("[stderr] ");
            s.push_str(self.stderr.trim_end());
        }
        if s.is_empty() {
            if self.timed_out {
                s.push_str("[timeout] command killed after exceeding its time limit");
            } else if !self.success {
                s.push_str(&format!(
                    "[exit code: {}]",
                    self.status_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".to_string())
                ));
            }
        }
        s
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Sandbox {
    pub fn from_env() -> Self {
        let bwrap = std::env::var_os("AGENT_BENCH_BWRAP")
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| which("bwrap"));
        let allow_unsandboxed = env_flag("AGENT_BENCH_ALLOW_UNSANDBOXED");
        let allow_network = env_flag("AGENT_BENCH_ALLOW_NETWORK");
        Self {
            bwrap,
            allow_unsandboxed,
            allow_network,
            default_max_output_bytes: 128 * 1024,
        }
    }

    pub fn with_bwrap(path: Option<PathBuf>, allow_unsandboxed: bool) -> Self {
        Self {
            bwrap: path,
            allow_unsandboxed,
            allow_network: false,
            default_max_output_bytes: 128 * 1024,
        }
    }

    pub fn is_isolated(&self) -> bool {
        self.bwrap.is_some()
    }

    pub fn bwrap_path(&self) -> Option<&Path> {
        self.bwrap.as_deref()
    }

    pub fn allow_network(&self) -> bool {
        self.allow_network
    }

    pub fn mode_label(&self) -> &'static str {
        if self.is_isolated() {
            "bwrap"
        } else if self.allow_unsandboxed {
            "unsafe-no-isolation"
        } else {
            "unavailable"
        }
    }

    pub fn run_bash(&self, command: &str, cwd: &Path, timeout: Duration) -> Result<ExecOutput> {
        self.run_bash_with(command, cwd, timeout, Vec::new(), Vec::new())
    }

    pub fn run_bash_with(
        &self,
        command: &str,
        cwd: &Path,
        timeout: Duration,
        extra_ro_binds: Vec<(PathBuf, PathBuf)>,
        extra_env: Vec<(String, String)>,
    ) -> Result<ExecOutput> {
        self.run(ExecRequest {
            program: "bash".to_string(),
            args: vec!["-c".to_string(), command.to_string()],
            cwd: cwd.to_path_buf(),
            timeout,
            extra_ro_binds,
            extra_env,
            max_output_bytes: self.default_max_output_bytes,
        })
    }

    pub fn run_python(
        &self,
        script: &str,
        cwd: &Path,
        timeout: Duration,
        extra_env: Vec<(String, String)>,
    ) -> Result<ExecOutput> {
        self.run(ExecRequest {
            program: "python3".to_string(),
            args: vec!["-B".to_string(), script.to_string()],
            cwd: cwd.to_path_buf(),
            timeout,
            extra_ro_binds: Vec::new(),
            extra_env,
            max_output_bytes: self.default_max_output_bytes,
        })
    }

    pub fn run(&self, req: ExecRequest) -> Result<ExecOutput> {
        let cwd = req
            .cwd
            .canonicalize()
            .with_context(|| format!("Sandbox cwd does not exist: {}", req.cwd.display()))?;
        if !cwd.is_dir() {
            bail!("Sandbox cwd is not a directory: {}", cwd.display());
        }

        let isolated = self.bwrap.is_some();
        if !isolated && !self.allow_unsandboxed {
            bail!(
                "Refusing to execute model-controlled commands without isolation: \
                 bubblewrap (bwrap) was not found. Install bwrap, or set \
                 AGENT_BENCH_ALLOW_UNSANDBOXED=1 to explicitly accept the risk."
            );
        }

        let env = sandbox_env(&cwd, &req.extra_env);
        let mut cmd = if let Some(ref bwrap) = self.bwrap {
            let mut c = Command::new(bwrap);
            c.arg("--die-with-parent").arg("--unshare-all");
            if self.allow_network {
                c.arg("--share-net");
            }
            // Host root is read-only. Only the workspace is writable.
            c.args(["--ro-bind", "/", "/"]);
            // Fresh /tmp; workspace is re-bound after this if it lives under /tmp.
            c.args(["--tmpfs", "/tmp"]);
            c.args(["--dev", "/dev"]);
            c.args(["--proc", "/proc"]);
            c.arg("--bind").arg(&cwd).arg(&cwd);
            for (src, dest) in &req.extra_ro_binds {
                let src = src.canonicalize().with_context(|| {
                    format!("Sandbox ro-bind source missing: {}", src.display())
                })?;
                let dest = dest.canonicalize().with_context(|| {
                    format!("Sandbox ro-bind destination missing: {}", dest.display())
                })?;
                if !dest.starts_with(&cwd) {
                    bail!(
                        "Sandbox ro-bind destination must be inside workspace: {}",
                        dest.display()
                    );
                }
                c.arg("--ro-bind").arg(src).arg(dest);
            }
            c.arg("--chdir").arg(&cwd);
            c.arg("--clearenv");
            for (k, v) in &env {
                c.arg("--setenv").arg(k).arg(v);
            }
            c.arg("--").arg(&req.program).args(&req.args);
            c.current_dir(&cwd);
            // The host-side bwrap process does not need the host environment.
            c.env_clear();
            c
        } else {
            let mut c = Command::new(&req.program);
            c.args(&req.args).current_dir(&cwd);
            c.env_clear();
            for (k, v) in &env {
                c.env(k, v);
            }
            c
        };

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn sandboxed process {}", req.program))?;
        let pid = child.id();

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;

        let max_bytes = req.max_output_bytes.max(1024);
        let out_thread = std::thread::spawn(move || read_capped(stdout, max_bytes));
        let err_thread = std::thread::spawn(move || read_capped(stderr, max_bytes));

        let timed_out = Arc::new(AtomicBool::new(false));
        let timeout_flag = Arc::clone(&timed_out);
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let watchdog = std::thread::spawn(move || {
            if done_rx.recv_timeout(req.timeout).is_err() {
                timeout_flag.store(true, Ordering::SeqCst);
                kill_process_group(pid);
            }
        });

        let status = child.wait();
        // The direct child is gone. Kill any background grandchildren that may
        // still hold the pipes open, then collect the (possibly truncated) output.
        let _ = done_tx.send(());
        kill_process_group(pid);
        let _ = watchdog.join();

        let status = status.context("Failed while waiting for sandboxed process")?;
        let stdout = out_thread.join().unwrap_or_default();
        let stderr = err_thread.join().unwrap_or_default();

        Ok(ExecOutput {
            status_code: status.code(),
            success: status.success() && !timed_out.load(Ordering::SeqCst),
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            timed_out: timed_out.load(Ordering::SeqCst),
            sandboxed: isolated,
        })
    }
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn which(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn sandbox_env(cwd: &Path, extra: &[(String, String)]) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string());
    let home = cwd.to_string_lossy().to_string();
    let mut push = |k: &str, v: String| env.push((k.to_string(), v));

    push("PATH", path);
    push("HOME", home.clone());
    push("TMPDIR", "/tmp".to_string());
    push("TMP", "/tmp".to_string());
    push("TEMP", "/tmp".to_string());
    push(
        "LANG",
        std::env::var("LANG").unwrap_or_else(|_| "C.UTF-8".to_string()),
    );
    push(
        "LC_ALL",
        std::env::var("LC_ALL").unwrap_or_else(|_| "C.UTF-8".to_string()),
    );
    push(
        "LC_CTYPE",
        std::env::var("LC_CTYPE").unwrap_or_else(|_| "C.UTF-8".to_string()),
    );
    push(
        "TERM",
        std::env::var("TERM").unwrap_or_else(|_| "dumb".to_string()),
    );
    push(
        "USER",
        std::env::var("USER").unwrap_or_else(|_| "sandbox".to_string()),
    );
    push(
        "LOGNAME",
        std::env::var("LOGNAME").unwrap_or_else(|_| "sandbox".to_string()),
    );
    push("SHELL", "/bin/bash".to_string());
    push("PYTHONNOUSERSITE", "1".to_string());
    push("PYTHONDONTWRITEBYTECODE", "1".to_string());
    push("PYTHONUNBUFFERED", "1".to_string());

    for (k, v) in extra {
        // Never allow the caller to inject loader/runtime hooks that could
        // escape the sandbox or re-import host state.
        if !is_safe_env_key(k) || is_dangerous_env_key(k) {
            continue;
        }
        if let Some(existing) = env.iter_mut().find(|(ek, _)| ek == k) {
            existing.1 = v.clone();
        } else {
            env.push((k.clone(), v.clone()));
        }
    }
    env
}

fn is_safe_env_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

fn is_dangerous_env_key(key: &str) -> bool {
    key.starts_with("LD_")
        || key.starts_with("DYLD_")
        || matches!(
            key,
            "PYTHONPATH"
                | "PYTHONSTARTUP"
                | "BASH_ENV"
                | "ENV"
                | "PERL5OPT"
                | "RUBYOPT"
                | "NODE_OPTIONS"
                | "GIT_CONFIG_GLOBAL"
                | "GIT_CONFIG_SYSTEM"
        )
}

fn read_capped(mut reader: impl Read, max_bytes: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if out.len() < max_bytes {
                    let take = n.min(max_bytes - out.len());
                    out.extend_from_slice(&buf[..take]);
                    if take < n {
                        truncated = true;
                    }
                } else {
                    truncated = true;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    if truncated {
        out.extend_from_slice(b"\n[output truncated]\n");
    }
    out
}

#[cfg(unix)]
fn kill_process_group(pid: u32) {
    if pid == 0 {
        return;
    }
    // Negative pid means "the process group led by pid".
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill_process_group(_pid: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox() -> Sandbox {
        Sandbox::from_env()
    }

    #[test]
    fn sandbox_clears_host_environment() {
        std::env::set_var("AGENT_BENCH_SECRET_SENTINEL", "leak-me");
        let sb = sandbox();
        if !sb.is_isolated() {
            return;
        }
        let out = sb
            .run_bash(
                "printf '%s' \"${AGENT_BENCH_SECRET_SENTINEL:-}\"",
                Path::new("."),
                Duration::from_secs(5),
            )
            .unwrap();
        assert_eq!(out.stdout.trim(), "");
    }

    #[test]
    fn sandbox_kills_background_processes_on_timeout() {
        let sb = sandbox();
        if !sb.is_isolated() {
            return;
        }
        let started = std::time::Instant::now();
        let out = sb
            .run_bash("sleep 3 &", Path::new("."), Duration::from_millis(250))
            .unwrap();
        assert!(out.timed_out || started.elapsed() < Duration::from_secs(2));
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "background process kept the command alive: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn sandbox_binds_workspace_read_write() {
        let sb = sandbox();
        if !sb.is_isolated() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = sb
            .run_bash(
                "printf ok > sandbox-write-test.txt && cat sandbox-write-test.txt",
                dir.path(),
                Duration::from_secs(5),
            )
            .unwrap();
        assert_eq!(out.stdout.trim(), "ok");
        assert!(dir.path().join("sandbox-write-test.txt").exists());
    }

    #[test]
    fn network_is_unshared_by_default() {
        let sb = sandbox();
        if !sb.is_isolated() || sb.allow_network() {
            return;
        }
        // This does not require an external network: opening a TCP socket is
        // enough to prove that the network namespace is isolated.
        let out = sb
            .run_bash(
                "python3 - <<'PY'\nimport socket\ntry:\n    s=socket.create_connection(('1.1.1.1',80),timeout=2)\n    print('NETWORK_OPEN')\nexcept Exception as e:\n    print('NETWORK_BLOCKED')\nPY",
                Path::new("."),
                Duration::from_secs(8),
            )
            .unwrap();
        assert!(
            out.stdout.contains("NETWORK_BLOCKED") || !out.success,
            "network unexpectedly reachable: {} {}",
            out.stdout,
            out.stderr
        );
    }
}
