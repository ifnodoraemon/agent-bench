use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Single-turn BFCL-style tool-call evaluator. Compares the function name and
/// expected argument values (BFCL stores each argument as a list of acceptable
/// values). It is intentionally strict: a missing expected argument fails.
#[derive(Default)]
pub struct ToolCallEvaluator;

impl ToolCallEvaluator {
    fn value_matches(actual: &Value, allowed: &Value) -> bool {
        if actual == allowed {
            return true;
        }
        if let (Some(a), Some(b)) = (actual.as_f64(), allowed.as_f64()) {
            return (a - b).abs() <= 1e-6 * b.abs().max(1.0);
        }
        if let (Some(a), Some(b)) = (actual.as_str(), allowed.as_str()) {
            return a.trim().eq_ignore_ascii_case(b.trim());
        }
        false
    }

    /// Match one candidate value. BFCL ground truth uses nested
    /// list-of-acceptable-values structures (for example
    /// `update_info: [{name: ["John Doe"], email: [...]}]`), so argument
    /// comparison must be recursive.
    fn match_candidate(actual: &Value, expected: &Value) -> bool {
        if let (Some(actual_obj), Some(expected_obj)) = (actual.as_object(), expected.as_object()) {
            return expected_obj.iter().all(|(key, allowed)| {
                let Some(actual_value) = actual_obj.get(key) else {
                    return false;
                };
                let matched = match allowed.as_array() {
                    Some(values) => values
                        .iter()
                        .any(|candidate| Self::match_candidate(actual_value, candidate)),
                    None => Self::match_candidate(actual_value, allowed),
                };
                matched
            });
        }
        if let (Some(actual_arr), Some(expected_arr)) = (actual.as_array(), expected.as_array()) {
            return actual_arr.len() == expected_arr.len()
                && actual_arr
                    .iter()
                    .zip(expected_arr)
                    .all(|(a, e)| Self::match_candidate(a, e));
        }
        Self::value_matches(actual, expected)
    }

    fn args_match(expected: &Value, actual: &Value) -> bool {
        let Some(expected_obj) = expected.as_object() else {
            return false;
        };
        let Some(actual_obj) = actual.as_object() else {
            return false;
        };
        for (key, allowed) in expected_obj {
            let Some(actual_value) = actual_obj.get(key) else {
                // Optional argument: ground truth marks omission with ""/null.
                let optional = allowed
                    .as_array()
                    .map(|values| values.iter().any(|v| v.is_null() || v.as_str() == Some("")))
                    .unwrap_or(false);
                if optional {
                    continue;
                }
                return false;
            };
            let matched = match allowed.as_array() {
                Some(values) => values
                    .iter()
                    .any(|candidate| Self::match_candidate(actual_value, candidate)),
                None => Self::match_candidate(actual_value, allowed),
            };
            if !matched {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl Evaluator for ToolCallEvaluator {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
        let Some(reference) = test_case.reference_answer.as_deref() else {
            return Ok(EvaluationResult::fail(
                "tool_call requires reference_answer with ground-truth calls",
            ));
        };
        let Ok(reference_json) = serde_json::from_str::<Value>(reference) else {
            return Ok(EvaluationResult::fail(
                "tool_call reference_answer is not valid JSON",
            ));
        };
        let Some(ground_truth) = reference_json.as_array() else {
            return Ok(EvaluationResult::fail(
                "tool_call reference_answer must be a JSON array",
            ));
        };
        let Some(calls) = response.tool_calls.as_deref() else {
            return Ok(EvaluationResult::fail(
                "Model returned no tool_calls for a tool_call case",
            ));
        };
        if calls.is_empty() {
            return Ok(EvaluationResult::fail(
                "Model returned an empty tool_calls list",
            ));
        }

        for entry in ground_truth {
            let Some(entry_obj) = entry.as_object() else {
                continue;
            };
            for (expected_name, expected_args) in entry_obj {
                for call in calls {
                    if call.function.name != *expected_name {
                        continue;
                    }
                    let actual_args = serde_json::from_str::<Value>(&call.function.arguments)
                        .unwrap_or(Value::Null);
                    if Self::args_match(expected_args, &actual_args) {
                        return Ok(EvaluationResult::pass(format!(
                            "Tool call matched: {expected_name}"
                        )));
                    }
                }
            }
        }

        let actual_names: Vec<String> = calls
            .iter()
            .map(|c| format!("{} {}", c.function.name, c.function.arguments))
            .collect();
        Ok(EvaluationResult::fail(format!(
            "No ground-truth tool call matched. Model calls: {}",
            actual_names.join("; ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{Category, EvaluationType};
    use crate::model::{FunctionCall, TokenUsage, ToolCall};
    use std::collections::HashMap;
    use std::time::Duration;

    fn case(reference: &str) -> TestCase {
        TestCase {
            id: "tool".to_string(),
            name: None,
            category: Category::Agent,
            difficulty: None,
            tags: vec![],
            prompt: "q".to_string(),
            system_prompt: None,
            reference_answer: Some(reference.to_string()),
            eval_type: EvaluationType::ToolCall,
            criteria: None,
            schema: None,
            test_code: None,
            tools: None,
            max_turns: None,
            metadata: HashMap::new(),
        }
    }

    fn response(name: &str, args: &str) -> ModelResponse {
        ModelResponse {
            text: String::new(),
            reasoning_content: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: name.to_string(),
                    arguments: args.to_string(),
                },
            }]),
            usage: TokenUsage::default(),
            ttft: None,
            total_duration: Duration::from_millis(1),
            tokens_per_second: 1.0,
            estimated_cost_usd: 0.0,
            finish_reason: None,
            raw_response: None,
        }
    }

    #[tokio::test]
    async fn matches_bfcl_ground_truth() {
        let eval = ToolCallEvaluator;
        let reference = r#"[{"solve_quadratic":{"a":[3],"b":[-11],"c":[-4],"root_type":["all"]}}]"#;
        let res = eval
            .evaluate(
                &case(reference),
                &response(
                    "solve_quadratic",
                    r#"{"a": 3, "b": -11, "c": -4, "root_type": "all"}"#,
                ),
            )
            .await
            .unwrap();
        assert!(res.passed, "{}", res.reason);
    }

    #[tokio::test]
    async fn accepts_omitted_optional_argument() {
        let eval = ToolCallEvaluator;
        let reference =
            r#"[{"calculate_velocity":{"distance":[50],"duration":[2],"unit":["","km/h"]}}]"#;
        let res = eval
            .evaluate(
                &case(reference),
                &response("calculate_velocity", r#"{"distance":50,"duration":2}"#),
            )
            .await
            .unwrap();
        assert!(res.passed, "{}", res.reason);
    }

    #[tokio::test]
    async fn matches_nested_bfcl_arguments() {
        let eval = ToolCallEvaluator;
        let reference = r#"[{"update_user_info":{"user_id":[43523],"update_info":[{"name":["John Doe"],"email":["johndoe@email.com"]}],"database":["CustomerInfo",""]}}]"#;
        let res = eval
            .evaluate(
                &case(reference),
                &response(
                    "update_user_info",
                    r#"{"user_id":43523,"update_info":{"name":"John Doe","email":"johndoe@email.com"},"database":"CustomerInfo"}"#,
                ),
            )
            .await
            .unwrap();
        assert!(res.passed, "{}", res.reason);
    }

    #[tokio::test]
    async fn rejects_wrong_args_or_no_tool_call() {
        let eval = ToolCallEvaluator;
        let reference = r#"[{"add":{"a":[1],"b":[2]}}]"#;
        assert!(
            !eval
                .evaluate(&case(reference), &response("add", r#"{"a":1,"b":3}"#))
                .await
                .unwrap()
                .passed
        );
        let mut no_call = response("add", r#"{"a":1,"b":2}"#);
        no_call.tool_calls = None;
        assert!(
            !eval
                .evaluate(&case(reference), &no_call)
                .await
                .unwrap()
                .passed
        );
    }
}
