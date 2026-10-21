use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use std::sync::LazyLock;

static NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?").expect("valid number regex"));

/// Extracts the final numeric answer. GSM8K-style `#### 18` is preferred; a
/// plain final number is accepted. This avoids requiring the model to return
/// only a number while still rejecting non-numeric prose.
#[derive(Default)]
pub struct NumericEvaluator;

impl NumericEvaluator {
    fn parse_number(text: &str) -> Option<f64> {
        let cleaned = text.trim().replace(',', "");
        cleaned.parse::<f64>().ok().filter(|v| v.is_finite())
    }

    fn candidate(text: &str) -> Option<f64> {
        if let Some(pos) = text.rfind("####") {
            let tail = &text[pos + 4..];
            if let Some(v) = NUMBER_RE
                .find_iter(tail)
                .filter_map(|m| m.as_str().parse::<f64>().ok())
                .last()
            {
                return Some(v);
            }
        }
        NUMBER_RE
            .find_iter(text)
            .filter_map(|m| m.as_str().parse::<f64>().ok())
            .last()
    }
}

#[async_trait]
impl Evaluator for NumericEvaluator {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
        let Some(reference) = test_case
            .reference_answer
            .as_deref()
            .and_then(Self::parse_number)
        else {
            return Ok(EvaluationResult::fail(
                "Numeric evaluator requires a numeric reference_answer",
            ));
        };
        let Some(candidate) = Self::candidate(&response.text) else {
            return Ok(EvaluationResult::fail(format!(
                "No number found in output: {}",
                response.text.chars().take(200).collect::<String>()
            )));
        };
        let tolerance = 1e-6 * reference.abs().max(1.0);
        if (candidate - reference).abs() <= tolerance {
            Ok(EvaluationResult::pass(format!(
                "Numeric match: {candidate} == {reference}"
            )))
        } else {
            Ok(EvaluationResult::fail(format!(
                "Numeric mismatch: expected {reference}, got {candidate}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{Category, EvaluationType};
    use crate::model::TokenUsage;
    use std::collections::HashMap;
    use std::time::Duration;

    fn case(reference: &str) -> TestCase {
        TestCase {
            id: "n".to_string(),
            name: None,
            category: Category::MathLogic,
            difficulty: None,
            tags: vec![],
            prompt: "p".to_string(),
            system_prompt: None,
            reference_answer: Some(reference.to_string()),
            eval_type: EvaluationType::Numeric,
            criteria: None,
            schema: None,
            test_code: None,
            tools: None,
            max_turns: None,
            metadata: HashMap::new(),
        }
    }

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

    #[tokio::test]
    async fn accepts_gsm8k_final_marker_and_plain_numbers() {
        let eval = NumericEvaluator;
        assert!(
            eval.evaluate(&case("18"), &response("reasoning...\n#### 18"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            eval.evaluate(&case("3.5"), &response("The answer is 3.5."))
                .await
                .unwrap()
                .passed
        );
    }

    #[tokio::test]
    async fn rejects_wrong_or_missing_numbers() {
        let eval = NumericEvaluator;
        assert!(
            !eval
                .evaluate(&case("18"), &response("#### 19"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            !eval
                .evaluate(&case("18"), &response("no numbers here"))
                .await
                .unwrap()
                .passed
        );
    }
}
