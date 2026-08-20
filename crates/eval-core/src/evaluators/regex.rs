use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;

#[derive(Default)]
pub struct RegexEvaluator;

impl RegexEvaluator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Evaluator for RegexEvaluator {
    async fn evaluate(&self, test_case: &TestCase, response: &ModelResponse) -> Result<EvaluationResult> {
        let pattern = match &test_case.criteria {
            Some(crit) => crit.as_str(),
            None => match &test_case.reference_answer {
                Some(ref_ans) => ref_ans.as_str(),
                None => {
                    return Ok(EvaluationResult::fail(
                        "No regex pattern found in criteria or reference_answer",
                    ));
                }
            },
        };

        match Regex::new(pattern) {
            Ok(re) => {
                if re.is_match(&response.text) {
                    Ok(EvaluationResult::pass(format!(
                        "Pattern /{}/ matched successfully",
                        pattern
                    )))
                } else {
                    Ok(EvaluationResult::fail(format!(
                        "Pattern /{}/ did not match output: '{}'",
                        pattern, response.text
                    )))
                }
            }
            Err(e) => Ok(EvaluationResult::fail(format!("Invalid regex pattern: {e}"))),
        }
    }
}
