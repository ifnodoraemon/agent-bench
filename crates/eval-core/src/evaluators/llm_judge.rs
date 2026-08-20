use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::{ChatMessage, ModelClient, ModelResponse};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeOutput {
    pub reasoning: String,
    pub score: f64, // 0.0 to 1.0
    pub passed: bool,
}

pub struct LlmJudgeEvaluator {
    judge_client: Arc<dyn ModelClient>,
    judge_prompt_template: String,
}

impl LlmJudgeEvaluator {
    pub fn new(judge_client: Arc<dyn ModelClient>) -> Self {
        let default_template = r#"You are an impartial, expert AI judge evaluating the quality of an LLM's response.

[TASK / PROMPT]:
{PROMPT}

[REFERENCE ANSWER / CRITERIA]:
{CRITERIA}

[MODEL RESPONSE TO EVALUATE]:
{RESPONSE}

Please evaluate the response against the criteria.
Output your evaluation in strict JSON format as follows:
```json
{
  "reasoning": "Step-by-step reasoning for why the response meets or fails the criteria",
  "score": 0.85, // Float between 0.0 (completely incorrect/unhelpful) and 1.0 (perfect)
  "passed": true // true if score >= 0.7, false otherwise
}
```
"#
        .to_string();

        Self {
            judge_client,
            judge_prompt_template: default_template,
        }
    }

    pub fn with_custom_template(mut self, template: impl Into<String>) -> Self {
        self.judge_prompt_template = template.into();
        self
    }
}

#[async_trait]
impl Evaluator for LlmJudgeEvaluator {
    async fn evaluate(&self, test_case: &TestCase, response: &ModelResponse) -> Result<EvaluationResult> {
        let criteria_text = test_case
            .criteria
            .clone()
            .or_else(|| test_case.reference_answer.clone())
            .unwrap_or_else(|| "Provide an accurate, clear, and helpful answer.".to_string());

        let formatted_prompt = self
            .judge_prompt_template
            .replace("{PROMPT}", &test_case.prompt)
            .replace("{CRITERIA}", &criteria_text)
            .replace("{RESPONSE}", &response.text);

        let messages = vec![ChatMessage::user(formatted_prompt)];
        let judge_resp = self.judge_client.chat_complete(&messages, None).await?;

        // Extract JSON from judge response
        if let Some(json_val) = super::schema::JsonSchemaEvaluator::extract_json(&judge_resp.text) {
            let score = json_val
                .get("score")
                .and_then(|s| s.as_f64())
                .unwrap_or(0.0);
            let passed = json_val
                .get("passed")
                .and_then(|p| p.as_bool())
                .unwrap_or(score >= 0.7);
            let reasoning = json_val
                .get("reasoning")
                .and_then(|r| r.as_str())
                .unwrap_or("No reasoning provided")
                .to_string();

            Ok(EvaluationResult {
                passed,
                score: score.clamp(0.0, 1.0),
                reason: format!("LLM-Judge ({:.2}/1.0): {}", score, reasoning),
                details: Some(json_val),
            })
        } else {
            Ok(EvaluationResult::fail(format!(
                "Failed to parse JSON from judge output:\n{}",
                judge_resp.text
            )))
        }
    }
}
