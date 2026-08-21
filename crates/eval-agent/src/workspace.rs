use super::env::SimulatedEnvironment;
use super::pi_tools::execute_pi_tool;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

pub struct WorkspaceEnv {
    temp_dir: Arc<Mutex<TempDir>>,
    custom_mock_tools: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for WorkspaceEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create temporary workspace directory")
    }
}

impl WorkspaceEnv {
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let env = Self {
            temp_dir: Arc::new(Mutex::new(temp_dir)),
            custom_mock_tools: Arc::new(Mutex::new(HashMap::new())),
        };

        // Pre-seed standard workspace environment files
        let root = env.workspace_path();
        let app_dir = root.join("app");
        let logs_dir = app_dir.join("logs");
        let _ = fs::create_dir_all(&logs_dir);

        let _ = fs::write(
            app_dir.join("config.yaml"),
            "server:\n  port: 8080\n  host: 0.0.0.0\ndatabase:\n  url: postgres://admin:secret@localhost:5432/app_db\n",
        );

        let _ = fs::write(
            logs_dir.join("error.log"),
            "[2026-08-20 04:12:01] ERROR ConnectionRefused: database host localhost:5432 unreachable\n[2026-08-20 04:12:05] FATAL Server shutdown abnormally\n",
        );

        Ok(env)
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.temp_dir.lock().unwrap().path().to_path_buf()
    }

    /// Pre-populate workspace with seed files, scripts, or project code
    pub fn setup_from_files(&self, files: &HashMap<String, String>) -> Result<()> {
        let root = self.workspace_path();
        for (rel_path, content) in files {
            let full_path = root.join(rel_path.trim_start_matches('/'));
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&full_path, content)
                .with_context(|| format!("Failed to write setup file: {}", full_path.display()))?;
        }
        Ok(())
    }

    /// Pre-populate custom mock tool outputs from test case metadata
    pub fn set_mock_tools(&self, mock_tools: HashMap<String, String>) {
        if let Ok(mut map) = self.custom_mock_tools.lock() {
            *map = mock_tools;
        }
    }

    /// Read a file from the workspace for test assertion
    pub fn read_file<P: AsRef<Path>>(&self, relative_path: P) -> Result<String> {
        let full_path = self.workspace_path().join(relative_path);
        let content = fs::read_to_string(full_path)?;
        Ok(content)
    }

    /// Run post-task automated verification script in the workspace (e.g. pytest or python test.py)
    pub fn run_verification(&self, verify_cmd: &str) -> Result<(bool, String)> {
        let root = self.workspace_path();
        let output = Command::new("timeout")
            .arg("60s")
            .arg("bash")
            .arg("-c")
            .arg(verify_cmd)
            .current_dir(&root)
            .output()
            .with_context(|| format!("Failed to execute verification command: {verify_cmd}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);
        let passed = output.status.success();

        let mut report = String::new();
        if exit_code == 124 {
            report.push_str("❌ Verification command TIMED OUT (exceeded 60s)\n");
        } else if passed {
            report.push_str("✅ Verification command PASSED (exit code 0)\n");
        } else {
            report.push_str(&format!(
                "❌ Verification command FAILED (exit code {exit_code})\n"
            ));
        }

        if !stdout.is_empty() {
            report.push_str(&format!("Stdout:\n{}\n", stdout.trim()));
        }
        if !stderr.is_empty() {
            report.push_str(&format!("Stderr:\n{}\n", stderr.trim()));
        }

        Ok((passed, report))
    }
}

impl SimulatedEnvironment for WorkspaceEnv {
    fn execute_tool(&self, name: &str, arguments_json: &str) -> Result<String> {
        // 1. Check if test case defined a static mock output for this tool
        if let Ok(guard) = self.custom_mock_tools.lock() {
            if let Some(mock_res) = guard.get(name) {
                return Ok(mock_res.clone());
            }
            // Check call signature match e.g. "calc(1+1)"
            let call_sig = format!("{name}({arguments_json})");
            if let Some(mock_res) = guard.get(&call_sig) {
                return Ok(mock_res.clone());
            }
        }

        // 2. Otherwise execute in real isolated workspace environment
        let root = self.workspace_path();
        execute_pi_tool(&root, name, arguments_json)
    }

    fn reset(&self) {
        // Workspace is temporary and isolated per task
    }
}
