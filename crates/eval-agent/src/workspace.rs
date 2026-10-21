use super::env::SimulatedEnvironment;
use super::path;
use super::pi_tools::execute_pi_tool_with_sandbox;
use anyhow::{anyhow, Context, Result};
use eval_core::sandbox::Sandbox;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

#[derive(Clone)]
pub struct WorkspaceEnv {
    temp_dir: Arc<Mutex<TempDir>>,
    custom_mock_tools: Arc<Mutex<HashMap<String, String>>>,
    /// Test files captured at setup time. They are restored immediately before
    /// verification, so a model cannot make tests pass by editing them.
    test_snapshot: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    sandbox: Sandbox,
}

impl Default for WorkspaceEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create temporary workspace directory")
    }
}

impl WorkspaceEnv {
    pub fn new() -> Result<Self> {
        Self::new_with_sandbox(Sandbox::from_env())
    }

    pub fn new_with_sandbox(sandbox: Sandbox) -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let env = Self {
            temp_dir: Arc::new(Mutex::new(temp_dir)),
            custom_mock_tools: Arc::new(Mutex::new(HashMap::new())),
            test_snapshot: Arc::new(Mutex::new(HashMap::new())),
            sandbox,
        };

        let root = env.workspace_path();
        let app_dir = root.join("app");
        let logs_dir = app_dir.join("logs");
        fs::create_dir_all(&logs_dir)?;

        fs::write(
            app_dir.join("config.yaml"),
            "server:\n  port: 8080\n  host: 0.0.0.0\ndatabase:\n  url: postgres://admin:secret@localhost:5432/app_db\n",
        )?;

        fs::write(
            logs_dir.join("error.log"),
            "[2026-08-20 04:12:01] ERROR ConnectionRefused: database host localhost:5432 unreachable\n[2026-08-20 04:12:05] FATAL Server shutdown abnormally\n",
        )?;

        seed_benchmark_db(&root)?;
        Ok(env)
    }

    pub fn sandbox_mode(&self) -> &'static str {
        self.sandbox.mode_label()
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.temp_dir.lock().unwrap().path().to_path_buf()
    }

    /// Seed files supplied by a dataset. Every path is workspace-relative and
    /// is opened with O_NOFOLLOW; `..`/absolute paths are rejected.
    pub fn setup_from_files(&self, files: &HashMap<String, String>) -> Result<()> {
        let root = self.workspace_path();
        for (rel_path, content) in files {
            path::write_no_follow(&root, rel_path, content.as_bytes())
                .with_context(|| format!("Failed to write setup file: {rel_path}"))?;
            if path::is_test_artifact(rel_path) {
                if let Ok(mut snap) = self.test_snapshot.lock() {
                    snap.insert(rel_path.clone(), content.as_bytes().to_vec());
                }
            }
        }
        Ok(())
    }

    pub fn set_mock_tools(&self, mock_tools: HashMap<String, String>) {
        if let Ok(mut map) = self.custom_mock_tools.lock() {
            *map = mock_tools;
        }
    }

    pub fn read_file<P: AsRef<Path>>(&self, relative_path: P) -> Result<String> {
        let rel = relative_path
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow!("Path is not valid UTF-8"))?;
        path::read_string_no_follow(&self.workspace_path(), rel)
    }

    /// Restore every test artifact captured at setup time and remove files a
    /// model may have added under `tests/` to shadow the real suite.
    pub fn restore_pristine_tests(&self) -> Result<()> {
        let root = self.workspace_path();
        path::remove_shadow_modules(&root);

        let snapshot = self
            .test_snapshot
            .lock()
            .map_err(|_| anyhow!("test snapshot lock poisoned"))?
            .clone();

        // Remove extra files under tests/ that were not part of the original
        // dataset, including an agent-created tests/conftest.py.
        let tests_dir = root.join("tests");
        if tests_dir.is_dir() {
            let mut extra = Vec::new();
            collect_regular_files(&tests_dir, &tests_dir, &mut extra)?;
            for rel in extra {
                let rel_from_root = format!("tests/{rel}");
                if !snapshot.contains_key(&rel_from_root) {
                    let _ = fs::remove_file(tests_dir.join(&rel));
                }
            }
        }

        for (rel, bytes) in &snapshot {
            path::write_no_follow(&root, rel, bytes)
                .with_context(|| format!("Failed to restore test artifact: {rel}"))?;
        }

        // Verify restoration. If an attacker can still change a test between
        // this check and the run, the sandbox is expected to prevent it.
        for (rel, bytes) in &snapshot {
            let current = path::read_string_no_follow(&root, rel)
                .with_context(|| format!("Failed to reread restored test artifact: {rel}"))?;
            if current.as_bytes() != bytes.as_slice() {
                return Err(anyhow!("Test artifact failed integrity check: {rel}"));
            }
        }
        Ok(())
    }

    fn test_snapshot_changed(&self) -> Result<bool> {
        let root = self.workspace_path();
        let snapshot = self
            .test_snapshot
            .lock()
            .map_err(|_| anyhow!("test snapshot lock poisoned"))?
            .clone();
        for (rel, bytes) in &snapshot {
            match path::read_string_no_follow(&root, rel) {
                Ok(current) if current.as_bytes() == bytes.as_slice() => {}
                _ => return Ok(true),
            }
        }
        Ok(false)
    }

    /// Run the dataset verification command in the isolated sandbox after
    /// restoring pristine tests. The command must be a shell command line.
    pub fn run_verification(&self, verify_cmd: &str) -> Result<(bool, String)> {
        self.restore_pristine_tests()?;
        let root = self.workspace_path();
        let effective_cmd = path::rewrite_app_alias(verify_cmd, &root);
        let out = self
            .sandbox
            .run_bash(&effective_cmd, &root, Duration::from_secs(60))?;

        if self.test_snapshot_changed()? {
            return Ok((
                false,
                "❌ Verification aborted: test artifacts were modified during the run".to_string(),
            ));
        }

        let mut report = String::new();
        if out.timed_out {
            report.push_str("❌ Verification command TIMED OUT (exceeded 60s)\n");
        } else if out.success {
            report.push_str(&format!(
                "✅ Verification command PASSED (exit code {})\n",
                out.status_code.unwrap_or(0)
            ));
        } else {
            report.push_str(&format!(
                "❌ Verification command FAILED (exit code {})\n",
                out.status_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            ));
        }
        if !out.stdout.trim().is_empty() {
            report.push_str(&format!("Stdout:\n{}\n", out.stdout.trim()));
        }
        if !out.stderr.trim().is_empty() {
            report.push_str(&format!("Stderr:\n{}\n", out.stderr.trim()));
        }
        report.push_str(&format!("Sandbox: {}\n", self.sandbox.mode_label()));
        Ok((out.success && !out.timed_out, report))
    }

    /// Initialize a git repository in the workspace.
    pub fn git_init_repo(&self) -> Result<()> {
        let root = self.workspace_path();
        let cmd = "git init -q && git config user.name 'Benchmark Agent' && \
                   git config user.email 'agent@benchmark.local' && \
                   git add -A && git commit -q -m 'Initial commit' --allow-empty";
        let out = self.sandbox.run_bash(cmd, &root, Duration::from_secs(30))?;
        if !out.success {
            return Err(anyhow!("git init failed: {}", out.combined()));
        }
        Ok(())
    }

    /// Return a git diff of the workspace modifications.
    pub fn git_diff(&self) -> Result<String> {
        let root = self.workspace_path();
        let out = self
            .sandbox
            .run_bash("git diff HEAD || true", &root, Duration::from_secs(30))?;
        Ok(out.combined())
    }

    /// List all relative file paths within the workspace directory. Symlinks
    /// are skipped so a model cannot make the evaluator traverse outside.
    pub fn list_files(&self, subpath: Option<&str>) -> Result<Vec<String>> {
        let root = self.workspace_path();
        let target = match subpath {
            Some(s) if !s.is_empty() && s != "." => path::safe_join(&root, s)?,
            _ => root.clone(),
        };
        if !target.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        fn walk(dir: &Path, root: &Path, acc: &mut Vec<String>, depth: usize) -> Result<()> {
            if depth > 32 || acc.len() >= 10_000 {
                return Ok(());
            }
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let Ok(meta) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if meta.file_type().is_symlink() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == ".git" {
                        continue;
                    }
                }
                if meta.is_dir() {
                    walk(&path, root, acc, depth + 1)?;
                } else if meta.is_file() {
                    if let Ok(rel) = path.strip_prefix(root) {
                        acc.push(rel.to_string_lossy().to_string());
                    }
                }
                if acc.len() >= 10_000 {
                    break;
                }
            }
            Ok(())
        }
        walk(&target, &root, &mut results, 0)?;
        results.sort();
        Ok(results)
    }
}

impl SimulatedEnvironment for WorkspaceEnv {
    fn execute_tool(&self, name: &str, arguments_json: &str) -> Result<String> {
        if let Ok(guard) = self.custom_mock_tools.lock() {
            if let Some(mock_res) = guard.get(name) {
                return Ok(mock_res.clone());
            }
            let call_sig = format!("{name}({arguments_json})");
            if let Some(mock_res) = guard.get(&call_sig) {
                return Ok(mock_res.clone());
            }
        }

        let root = self.workspace_path();
        execute_pi_tool_with_sandbox(&root, name, arguments_json, &self.sandbox)
    }

    fn reset(&self) {
        // Workspace is temporary and isolated per task.
    }
}

fn collect_regular_files(base: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    if out.len() >= 10_000 {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            collect_regular_files(base, &path, out)?;
        } else if meta.is_file() {
            if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
    }
    Ok(())
}

/// Create a small real SQLite database for the `database_query` tool. This is
/// seed data, not a text fixture: queries execute against SQLite with a
/// read-only connection.
pub(crate) fn seed_benchmark_db(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("data"))?;
    let conn = Connection::open(root.join("data/bench.db"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            balance REAL NOT NULL
        );
        CREATE TABLE IF NOT EXISTS orders (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            amount REAL NOT NULL,
            status TEXT NOT NULL
        );
        DELETE FROM users;
        DELETE FROM orders;
        INSERT INTO users (id, name, role, balance) VALUES
            (1, 'Alice', 'admin', 5000),
            (2, 'Bob', 'user', 250),
            (3, 'Charlie', 'user', 0);
        INSERT INTO orders (id, user_id, amount, status) VALUES
            (1, 1, 120.5, 'paid'),
            (2, 1, 80.0, 'pending'),
            (3, 2, 42.0, 'paid');",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_parent_dir_in_setup() {
        let env = WorkspaceEnv::new_with_sandbox(Sandbox::with_bwrap(None, true)).unwrap();
        let mut files = HashMap::new();
        files.insert("../escape.txt".to_string(), "x".to_string());
        assert!(env.setup_from_files(&files).is_err());
    }

    #[test]
    fn restores_test_snapshot_and_removes_shadow_module() {
        let env = WorkspaceEnv::new_with_sandbox(Sandbox::with_bwrap(None, true)).unwrap();
        let mut files = HashMap::new();
        files.insert(
            "tests/test_math.py".to_string(),
            "assert calculate() == 4\n".to_string(),
        );
        files.insert(
            "src/math.py".to_string(),
            "def calculate():\n    return 5\n".to_string(),
        );
        env.setup_from_files(&files).unwrap();

        let root = env.workspace_path();
        fs::write(root.join("tests/test_math.py"), "assert True\n").unwrap();
        fs::write(root.join("unittest.py"), "raise SystemExit(0)\n").unwrap();

        env.restore_pristine_tests().unwrap();
        let restored = fs::read_to_string(root.join("tests/test_math.py")).unwrap();
        assert!(restored.contains("calculate() == 4"));
        assert!(!root.join("unittest.py").exists());
    }

    #[test]
    fn read_file_rejects_traversal() {
        let env = WorkspaceEnv::new_with_sandbox(Sandbox::with_bwrap(None, true)).unwrap();
        assert!(env.read_file("../etc/passwd").is_err());
    }
}
