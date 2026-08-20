use super::SimulatedEnvironment;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

pub struct RealSubprocessBashEnv {
    temp_dir: Arc<Mutex<TempDir>>,
}

impl Default for RealSubprocessBashEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create temporary sandbox directory")
    }
}

impl RealSubprocessBashEnv {
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let app_dir = temp_dir.path().join("app");
        let logs_dir = app_dir.join("logs");
        std::fs::create_dir_all(&logs_dir)?;

        std::fs::write(
            app_dir.join("config.yaml"),
            "server:\n  port: 8080\n  host: 0.0.0.0\ndatabase:\n  url: postgres://admin:secret@localhost:5432/app_db\n",
        )?;

        std::fs::write(
            logs_dir.join("error.log"),
            "[2026-08-20 04:12:01] ERROR ConnectionRefused: database host localhost:5432 unreachable\n[2026-08-20 04:12:05] FATAL Server shutdown abnormally\n",
        )?;

        Ok(Self {
            temp_dir: Arc::new(Mutex::new(temp_dir)),
        })
    }

    pub fn sandbox_path(&self) -> PathBuf {
        self.temp_dir.lock().unwrap().path().to_path_buf()
    }

    pub fn write_file<P: AsRef<Path>>(&self, relative_path: P, content: &str) -> Result<()> {
        let full_path = self.sandbox_path().join(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full_path, content)?;
        Ok(())
    }

    pub fn read_file<P: AsRef<Path>>(&self, relative_path: P) -> Result<String> {
        let full_path = self.sandbox_path().join(relative_path);
        let content = std::fs::read_to_string(full_path)?;
        Ok(content)
    }
}

impl SimulatedEnvironment for RealSubprocessBashEnv {
    fn execute_tool(&self, name: &str, arguments_json: &str) -> Result<String> {
        let parsed_args: Value = serde_json::from_str(arguments_json)
            .map_err(|e| anyhow!("Invalid JSON arguments for tool '{name}': {e}"))?;

        let cwd = self.sandbox_path();

        match name {
            "bash" | "sh" | "terminal" => {
                let cmd = parsed_args
                    .get("command")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");

                if cmd.trim().is_empty() {
                    return Ok("Error: Empty bash command provided.".to_string());
                }

                let output = Command::new("bash")
                    .arg("-c")
                    .arg(cmd)
                    .current_dir(&cwd)
                    .output()?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                let mut res = String::new();
                if !stdout.is_empty() {
                    res.push_str(stdout.trim_end());
                }
                if !stderr.is_empty() {
                    if !res.is_empty() {
                        res.push('\n');
                    }
                    res.push_str(&format!("[stderr]: {}", stderr.trim_end()));
                }
                if exit_code != 0 && res.is_empty() {
                    res.push_str(&format!("[Exit code: {exit_code}]"));
                }
                if res.is_empty() {
                    res.push_str("(Command executed with no output, exit code 0)");
                }

                Ok(res)
            }
            "search_web" => {
                let query = parsed_args
                    .get("query")
                    .and_then(|q| q.as_str())
                    .unwrap_or("");
                Ok(format!("[Web Search for '{query}']: Official docs and facts returned."))
            }
            "calculator" => {
                let expr = parsed_args
                    .get("expression")
                    .and_then(|e| e.as_str())
                    .unwrap_or("");
                let py_cmd = format!("print({expr})");
                let output = Command::new("python3")
                    .arg("-c")
                    .arg(&py_cmd)
                    .output();
                if let Ok(out) = output {
                    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
                } else {
                    Ok(format!("Calculated: {expr}"))
                }
            }
            other => Err(anyhow!("Unknown tool in RealSubprocessBashEnv: {other}")),
        }
    }

    fn reset(&self) {
        // TempDir cleaned up on drop
    }
}
