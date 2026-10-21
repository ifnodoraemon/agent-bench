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
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
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

        let re = match Regex::new(pattern) {
            Ok(re) => re,
            Err(e) => {
                return Ok(EvaluationResult::fail(format!(
                    "Invalid regex pattern: {e}"
                )))
            }
        };
        if !re.is_match(&response.text) {
            return Ok(EvaluationResult::fail(format!(
                "Pattern /{}/ did not match output: '{}'",
                pattern, response.text
            )));
        }

        // Optional negative constraints, e.g. "must contain fruit/sweet but
        // must not mention banana". Rust's regex crate has no look-around, so
        // negative constraints are represented as separate patterns.
        if let Some(value) = test_case.metadata.get("must_not_match") {
            let patterns: Vec<&str> = match value {
                serde_json::Value::String(s) => vec![s.as_str()],
                serde_json::Value::Array(a) => a.iter().filter_map(|v| v.as_str()).collect(),
                _ => Vec::new(),
            };
            for forbidden in patterns {
                let Ok(fre) = Regex::new(forbidden) else {
                    return Ok(EvaluationResult::fail(format!(
                        "Invalid must_not_match regex: {forbidden}"
                    )));
                };
                if fre.is_match(&response.text) {
                    return Ok(EvaluationResult::fail(format!(
                        "Output matched forbidden pattern /{forbidden}/"
                    )));
                }
            }
        }

        Ok(EvaluationResult::pass(format!(
            "Pattern /{}/ matched successfully{}",
            pattern,
            if test_case.metadata.contains_key("must_not_match") {
                " (negative constraints satisfied)"
            } else {
                ""
            }
        )))
    }
}
