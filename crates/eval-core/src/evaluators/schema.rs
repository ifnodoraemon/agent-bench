use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use jsonschema::JSONSchema;
use serde_json::Value;

#[derive(Default)]
pub struct JsonSchemaEvaluator;

impl JsonSchemaEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Extract JSON substring from raw response text (handles markdown ```json blocks)
    pub fn extract_json(text: &str) -> Option<Value> {
        let trimmed = text.trim();
        // 1. Try parsing whole text
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            return Some(v);
        }

        // 2. Try extract from ```json ... ```
        if let Some(start_idx) = trimmed.find("```json") {
            let after_start = &trimmed[start_idx + 7..];
            if let Some(end_idx) = after_start.find("```") {
                let code = after_start[..end_idx].trim();
                if let Ok(v) = serde_json::from_str::<Value>(code) {
                    return Some(v);
                }
            }
        }

        // 3. Try extract from first '{' to last '}' or '[' to ']'
        if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
            if s < e {
                if let Ok(v) = serde_json::from_str::<Value>(&trimmed[s..=e]) {
                    return Some(v);
                }
            }
        }
        if let (Some(s), Some(e)) = (trimmed.find('['), trimmed.rfind(']')) {
            if s < e {
                if let Ok(v) = serde_json::from_str::<Value>(&trimmed[s..=e]) {
                    return Some(v);
                }
            }
        }

        None
    }
}

#[async_trait]
impl Evaluator for JsonSchemaEvaluator {
    async fn evaluate(&self, test_case: &TestCase, response: &ModelResponse) -> Result<EvaluationResult> {
        let schema_val = match &test_case.schema {
            Some(s) => s,
            None => {
                return Ok(EvaluationResult::fail("No schema specified in test case"));
            }
        };

        let json_data = match Self::extract_json(&response.text) {
            Some(j) => j,
            None => {
                return Ok(EvaluationResult::fail(
                    "Failed to extract valid JSON from model response",
                ));
            }
        };

        let compiled_schema = match JSONSchema::compile(schema_val) {
            Ok(s) => s,
            Err(e) => {
                return Ok(EvaluationResult::fail(format!(
                    "Invalid JSON schema in test case: {e}"
                )));
            }
        };

        let validation_result = compiled_schema.validate(&json_data);
        if let Err(errors) = validation_result {
            let err_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
            Ok(EvaluationResult::fail(format!(
                "JSON schema validation failed: {}",
                err_msgs.join("; ")
            )))
        } else {
            Ok(EvaluationResult::pass(
                "JSON format and schema validated successfully",
            ))
        }
    }
}
