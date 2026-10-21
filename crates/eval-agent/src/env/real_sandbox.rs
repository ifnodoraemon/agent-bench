use super::SimulatedEnvironment;
use crate::{calc, path, pi_tools};
use anyhow::{anyhow, Result};
use eval_core::sandbox::Sandbox;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

pub struct RealSubprocessBashEnv {
    temp_dir: Arc<Mutex<TempDir>>,
    sandbox: Sandbox,
}

impl Default for RealSubprocessBashEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create temporary sandbox directory")
    }
}

impl RealSubprocessBashEnv {
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let env = Self {
            temp_dir: Arc::new(Mutex::new(temp_dir)),
            sandbox: Sandbox::from_env(),
        };
        let app_dir = env.sandbox_path().join("app");
        let logs_dir = app_dir.join("logs");
        std::fs::create_dir_all(&logs_dir)?;

        path::write_no_follow(
            &env.sandbox_path(),
            "app/config.yaml",
            b"server:\n  port: 8080\n  host: 0.0.0.0\ndatabase:\n  url: postgres://admin:secret@localhost:5432/app_db\n",
        )?;

        path::write_no_follow(
            &env.sandbox_path(),
            "app/logs/error.log",
            b"[2026-08-20 04:12:01] ERROR ConnectionRefused: database host localhost:5432 unreachable\n[2026-08-20 04:12:05] FATAL Server shutdown abnormally\n",
        )?;

        crate::workspace::seed_benchmark_db(&env.sandbox_path())?;
        Ok(env)
    }

    pub fn sandbox_path(&self) -> PathBuf {
        self.temp_dir.lock().unwrap().path().to_path_buf()
    }

    pub fn write_file<P: AsRef<Path>>(&self, relative_path: P, content: &str) -> Result<()> {
        let rel = relative_path
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow!("Path is not valid UTF-8"))?;
        path::write_no_follow(&self.sandbox_path(), rel, content.as_bytes())?;
        Ok(())
    }

    pub fn read_file<P: AsRef<Path>>(&self, relative_path: P) -> Result<String> {
        let rel = relative_path
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow!("Path is not valid UTF-8"))?;
        path::read_string_no_follow(&self.sandbox_path(), rel)
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

                let effective_cmd = path::rewrite_app_alias(cmd, &cwd);
                let out = self
                    .sandbox
                    .run_bash(&effective_cmd, &cwd, Duration::from_secs(15))?;
                if out.timed_out {
                    return Ok(
                        "Error: Command execution timed out (exceeded 15s limit).".to_string()
                    );
                }
                let mut res = out.combined();
                if !out.success {
                    res.push_str(&format!(
                        "\n[exit code: {}]",
                        out.status_code
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "signal".to_string())
                    ));
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
                pi_tools::search_web_tool(query)
            }
            "database_query" | "query_database" => {
                pi_tools::database_query_tool(&self.sandbox_path(), &parsed_args)
            }
            "calculator" | "calculate" | "calc" => {
                let expr = parsed_args
                    .get("expression")
                    .and_then(|e| e.as_str())
                    .unwrap_or("");
                Ok(calc::evaluate_to_string(expr)?)
            }
            other => Err(anyhow!("Unknown tool in RealSubprocessBashEnv: {other}")),
        }
    }

    fn reset(&self) {
        // TempDir cleaned up on drop
    }
}
