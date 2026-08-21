use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

/// Verification outcome from a sandbox post-task check
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationOutcome {
    pub passed: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub details: String,
}

/// Abstract sandbox driver for isolating agent execution environments (Local, Docker, etc.)
#[async_trait]
pub trait SandboxDriver: Send + Sync {
    /// Setup initial workspace files
    fn setup_files(&self, files: &HashMap<String, String>) -> Result<()>;

    /// Root filesystem path of the sandbox (if local)
    fn workspace_path(&self) -> PathBuf;

    /// Execute a tool within the sandbox environment
    fn execute_tool(&self, tool_name: &str, arguments_json: &str) -> Result<String>;

    /// Run post-task automated verification script in the sandbox
    fn run_verification(&self, verify_cmd: &str) -> Result<VerificationOutcome>;
}
