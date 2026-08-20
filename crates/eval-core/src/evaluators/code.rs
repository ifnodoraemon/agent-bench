use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;

pub struct CodeSandboxEvaluator {
    pub language: String, // "python" | "bash" | "rust"
    pub timeout: Duration,
}

impl Default for CodeSandboxEvaluator {
    fn default() -> Self {
        Self {
            language: "python".to_string(),
            timeout: Duration::from_secs(10),
        }
    }
}

impl CodeSandboxEvaluator {
    pub fn new(language: impl Into<String>, timeout: Duration) -> Self {
        Self {
            language: language.into(),
            timeout,
        }
    }

    /// Extract code snippet from markdown ```python ... ```
    pub fn extract_code(text: &str, language: &str) -> String {
        let pattern_lang = format!("```{language}");
        if let Some(start_idx) = text.find(&pattern_lang) {
            let after_start = &text[start_idx + pattern_lang.len()..];
            if let Some(end_idx) = after_start.find("```") {
                return after_start[..end_idx].trim().to_string();
            }
        }
        if let Some(start_idx) = text.find("```") {
            let after_start = &text[start_idx + 3..];
            // Skip optional language line
            let code_start = after_start.find('\n').map(|i| i + 1).unwrap_or(0);
            let code_body = &after_start[code_start..];
            if let Some(end_idx) = code_body.find("```") {
                return code_body[..end_idx].trim().to_string();
            }
        }
        text.trim().to_string()
    }
}

#[async_trait]
impl Evaluator for CodeSandboxEvaluator {
    async fn evaluate(&self, test_case: &TestCase, response: &ModelResponse) -> Result<EvaluationResult> {
        let test_code = match &test_case.test_code {
            Some(tc) => tc.as_str(),
            None => {
                return Ok(EvaluationResult::fail("Missing test_code in test case"));
            }
        };

        let generated_code = Self::extract_code(&response.text, &self.language);
        let full_script = format!("{}\n\n# --- BENCHMARK TESTS ---\n{}", generated_code, test_code);

        let temp_dir = tempfile::tempdir()?;
        let script_file = temp_dir.path().join("test_run.py");
        tokio::fs::write(&script_file, &full_script).await?;

        let mut run_cmd = match self.language.as_str() {
            "python" | "python3" | "py" => {
                let mut cmd = Command::new("python3");
                cmd.arg(&script_file);
                cmd
            }
            "bash" | "sh" => {
                let mut cmd = Command::new("bash");
                cmd.arg(&script_file);
                cmd
            }
            other => {
                return Ok(EvaluationResult::fail(format!(
                    "Unsupported sandbox language: {other}"
                )));
            }
        };

        let child = run_cmd
            .current_dir(temp_dir.path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let execution = tokio::time::timeout(self.timeout, child.wait_with_output()).await;

        match execution {
            Ok(Ok(output)) => {
                if output.status.success() {
                    Ok(EvaluationResult::pass("Code passed all unit test assertions"))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(EvaluationResult::fail(format!(
                        "Test execution failed (exit code {}):\nStderr: {}\nStdout: {}",
                        output.status.code().unwrap_or(-1),
                        stderr.trim(),
                        stdout.trim()
                    )))
                }
            }
            Ok(Err(e)) => Ok(EvaluationResult::fail(format!("Failed to execute process: {e}"))),
            Err(_) => Ok(EvaluationResult::fail(format!(
                "Execution timed out (> {}s)",
                self.timeout.as_secs()
            ))),
        }
    }
}
