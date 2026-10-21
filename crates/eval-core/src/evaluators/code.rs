use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use crate::sandbox::Sandbox;
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

pub struct CodeSandboxEvaluator {
    pub language: String, // currently "python" (other languages fail closed)
    pub timeout: Duration,
}

impl Default for CodeSandboxEvaluator {
    fn default() -> Self {
        Self {
            language: "python".to_string(),
            timeout: Duration::from_secs(15),
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

    /// Extract code snippet from markdown ```python ... ```.
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
            let code_start = after_start.find('\n').map(|i| i + 1).unwrap_or(0);
            let code_body = &after_start[code_start..];
            if let Some(end_idx) = code_body.find("```") {
                return code_body[..end_idx].trim().to_string();
            }
        }
        text.trim().to_string()
    }
}

const RUNNER: &str = r#"
import os
import sys

class BlockedExit(RuntimeError):
    pass

def _blocked(*args, **kwargs):
    raise BlockedExit("process exit is not allowed during evaluation")

# Prevent the generated code from silently skipping the appended tests.
sys.exit = _blocked
os._exit = _blocked

def _run(path, name, namespace):
    with open(path, "r", encoding="utf-8") as handle:
        source = handle.read()
    code = compile(source, name, "exec")
    exec(code, namespace)

try:
    ns = {"__name__": "__main__", "__file__": "solution.py"}
    _run("solution.py", "solution.py", ns)
    _run("tests.py", "tests.py", ns)
except SystemExit as exc:
    raise BlockedExit("SystemExit(%r) is not allowed" % (exc.code,))
"#;

#[async_trait]
impl Evaluator for CodeSandboxEvaluator {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
        let test_code = match &test_case.test_code {
            Some(tc) if !tc.trim().is_empty() => tc.as_str(),
            _ => return Ok(EvaluationResult::fail("Missing test_code in test case")),
        };

        if self.language != "python" && self.language != "python3" && self.language != "py" {
            return Ok(EvaluationResult::fail(format!(
                "Unsupported sandbox language '{}' (only python is isolated)",
                self.language
            )));
        }

        let generated_code = Self::extract_code(&response.text, "python");
        if generated_code.trim().is_empty() {
            return Ok(EvaluationResult::fail("Model response contained no code"));
        }

        let temp_dir = tempfile::tempdir()?;
        tokio::fs::write(temp_dir.path().join("solution.py"), &generated_code).await?;
        tokio::fs::write(temp_dir.path().join("tests.py"), test_code).await?;
        tokio::fs::write(temp_dir.path().join("runner.py"), RUNNER).await?;

        let dir = temp_dir.path().to_path_buf();
        let sandbox = Sandbox::from_env();
        let timeout = self.timeout;
        let run = tokio::task::spawn_blocking(move || {
            sandbox.run_python("runner.py", &dir, timeout, Vec::new())
        })
        .await;

        let output = match run {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Ok(EvaluationResult::fail(format!("Sandbox error: {e}"))),
            Err(join_err) => {
                return Ok(EvaluationResult::fail(format!(
                    "Sandbox worker panicked: {join_err}"
                )))
            }
        };

        if output.timed_out {
            return Ok(EvaluationResult::fail(format!(
                "Code execution timed out after {}s",
                self.timeout.as_secs()
            )));
        }

        if output.success {
            Ok(EvaluationResult::pass(format!(
                "Code passed all tests in isolated {} sandbox",
                if output.sandboxed { "bwrap" } else { "unsafe" }
            )))
        } else {
            let mut detail = output.combined();
            if detail.len() > 8_000 {
                detail.truncate(8_000);
                detail.push_str("\n[truncated]");
            }
            Ok(EvaluationResult::fail(format!(
                "Test execution failed (exit code {}):\n{}",
                output
                    .status_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string()),
                detail
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ModelResponse, TokenUsage};

    fn response(text: &str) -> ModelResponse {
        ModelResponse {
            text: text.to_string(),
            reasoning_content: None,
            tool_calls: None,
            usage: TokenUsage::default(),
            ttft: None,
            total_duration: Duration::from_millis(1),
            tokens_per_second: 1.0,
            estimated_cost_usd: 0.0,
            finish_reason: None,
            raw_response: None,
        }
    }

    fn case(test_code: &str) -> TestCase {
        TestCase {
            id: "code".to_string(),
            name: None,
            category: crate::dataset::Category::Foundation,
            difficulty: None,
            tags: vec![],
            prompt: "write code".to_string(),
            system_prompt: None,
            reference_answer: None,
            eval_type: crate::dataset::EvaluationType::CodeExecution,
            criteria: None,
            schema: None,
            test_code: Some(test_code.to_string()),
            tools: None,
            max_turns: None,
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn passes_real_tests() {
        let evaluator = CodeSandboxEvaluator::default();
        let res = evaluator
            .evaluate(
                &case("assert add(2, 3) == 5\n"),
                &response("```python\ndef add(a, b):\n    return a + b\n```"),
            )
            .await
            .unwrap();
        assert!(res.passed, "{}", res.reason);
    }

    #[tokio::test]
    async fn blocks_sys_exit_bypass() {
        let evaluator = CodeSandboxEvaluator::default();
        let res = evaluator
            .evaluate(
                &case("assert False, 'tests must run'\n"),
                &response("```python\nimport sys\nsys.exit(0)\n```"),
            )
            .await
            .unwrap();
        assert!(!res.passed, "{}", res.reason);
    }

    #[tokio::test]
    async fn blocks_os_exit_bypass() {
        let evaluator = CodeSandboxEvaluator::default();
        let res = evaluator
            .evaluate(
                &case("assert False, 'tests must run'\n"),
                &response("```python\nimport os\nos._exit(0)\n```"),
            )
            .await
            .unwrap();
        assert!(!res.passed, "{}", res.reason);
    }

    #[tokio::test]
    async fn test_failure_is_reported() {
        let evaluator = CodeSandboxEvaluator::default();
        let res = evaluator
            .evaluate(
                &case("assert add(2, 3) == 6\n"),
                &response("```python\ndef add(a, b):\n    return a + b\n```"),
            )
            .await
            .unwrap();
        assert!(!res.passed);
    }
}
