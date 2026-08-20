use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Default)]
pub struct ExactMatchEvaluator {
    pub case_sensitive: bool,
    pub trim_whitespace: bool,
}

impl ExactMatchEvaluator {
    pub fn new(case_sensitive: bool, trim_whitespace: bool) -> Self {
        Self {
            case_sensitive,
            trim_whitespace,
        }
    }
}

#[async_trait]
impl Evaluator for ExactMatchEvaluator {
    async fn evaluate(&self, test_case: &TestCase, response: &ModelResponse) -> Result<EvaluationResult> {
        let reference = match &test_case.reference_answer {
            Some(ref_ans) => ref_ans.as_str(),
            None => {
                return Ok(EvaluationResult::fail("Missing reference_answer in test case"));
            }
        };

        let mut output = response.text.clone();
        let mut target = reference.to_string();

        if self.trim_whitespace {
            output = output.trim().to_string();
            target = target.trim().to_string();
        }

        if !self.case_sensitive {
            output = output.to_lowercase();
            target = target.to_lowercase();
        }

        if output == target {
            Ok(EvaluationResult::pass("Exact match succeeded"))
        } else if output.contains(&target) {
            // If output contains target answer (e.g. within a sentence)
            Ok(EvaluationResult::partial(
                0.8,
                format!("Output contains reference answer (Target: '{target}')"),
            ))
        } else {
            Ok(EvaluationResult::fail(format!(
                "Mismatch. Expected: '{target}', got: '{output}'"
            )))
        }
    }
}
