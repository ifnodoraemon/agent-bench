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
}

impl Default for WorkspaceEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create temporary workspace directory")
    }
}

impl WorkspaceEnv {
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self {
            temp_dir: Arc::new(Mutex::new(temp_dir)),
        })
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

    /// Read a file from the workspace for test assertion
    pub fn read_file<P: AsRef<Path>>(&self, relative_path: P) -> Result<String> {
        let full_path = self.workspace_path().join(relative_path);
        let content = fs::read_to_string(full_path)?;
        Ok(content)
    }

    /// Run post-task automated verification script in the workspace (e.g. pytest or python test.py)
    pub fn run_verification(&self, verify_cmd: &str) -> Result<(bool, String)> {
        let root = self.workspace_path();
        let output = Command::new("bash")
            .arg("-c")
            .arg(verify_cmd)
            .current_dir(&root)
            .output()
            .with_context(|| format!("Failed to execute verification command: {verify_cmd}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let passed = output.status.success();

        let mut report = String::new();
        if passed {
            report.push_str("✅ Verification command PASSED (exit code 0)\n");
        } else {
            report.push_str(&format!(
                "❌ Verification command FAILED (exit code {})\n",
                output.status.code().unwrap_or(-1)
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
        let root = self.workspace_path();
        execute_pi_tool(&root, name, arguments_json)
    }

    fn reset(&self) {
        // Workspace is temporary and isolated per task
    }
}
