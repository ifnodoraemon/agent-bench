use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use std::sync::LazyLock;

static ANSWER_LETTER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:final\s+answer|answer|option|choice)\s*(?:is|:)?\s*\(?([A-J])\)?")
        .expect("valid answer-letter regex")
});
static SINGLE_LETTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\(?([A-J])\)?[.)]?\s*$").expect("valid single-letter regex"));
static ANY_LETTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([A-J])\b").expect("valid any-letter regex"));

/// Multiple-choice evaluator for MMLU/MMLU-Pro/TruthfulQA style questions.
/// It parses a single option letter from the response and compares it with the
/// reference letter; it does not do substring matching on option text.
#[derive(Default)]
pub struct MultipleChoiceEvaluator;

impl MultipleChoiceEvaluator {
    fn extract_letter(text: &str) -> Option<char> {
        if let Some(cap) = ANSWER_LETTER_RE.captures(text) {
            return cap.get(1).and_then(|m| m.as_str().chars().next());
        }
        if let Some(cap) = SINGLE_LETTER_RE.captures(text) {
            return cap.get(1).and_then(|m| m.as_str().chars().next());
        }
        ANY_LETTER_RE
            .captures_iter(text)
            .last()
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().chars().next())
    }
}

#[async_trait]
impl Evaluator for MultipleChoiceEvaluator {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
        let Some(reference) = test_case
            .reference_answer
            .as_deref()
            .map(|s| s.trim().to_ascii_uppercase())
        else {
            return Ok(EvaluationResult::fail(
                "MultipleChoice evaluator requires reference_answer",
            ));
        };
        if reference.len() != 1 {
            return Ok(EvaluationResult::fail(format!(
                "Reference answer is not a single letter: {reference}"
            )));
        }
        let reference_char = reference.chars().next().unwrap_or('?');
        let Some(candidate) = Self::extract_letter(&response.text) else {
            return Ok(EvaluationResult::fail(format!(
                "No option letter found in output: {}",
                response.text.chars().take(200).collect::<String>()
            )));
        };
        if candidate.eq_ignore_ascii_case(&reference_char) {
            Ok(EvaluationResult::pass(format!(
                "Multiple-choice match: {candidate}"
            )))
        } else {
            Ok(EvaluationResult::fail(format!(
                "Multiple-choice mismatch: expected {reference_char}, got {candidate}"
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
            id: "mc".to_string(),
            name: None,
            category: Category::Foundation,
            difficulty: None,
            tags: vec![],
            prompt: "q".to_string(),
            system_prompt: None,
            reference_answer: Some(reference.to_string()),
            eval_type: EvaluationType::MultipleChoice,
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
    async fn extracts_letter_forms() {
        let eval = MultipleChoiceEvaluator;
        for text in ["B", "Answer: B", "Final answer is (B).", "B. some option"] {
            let res = eval.evaluate(&case("B"), &response(text)).await.unwrap();
            assert!(res.passed, "{text} -> {}", res.reason);
        }
    }

    #[tokio::test]
    async fn rejects_wrong_letter() {
        let eval = MultipleChoiceEvaluator;
        assert!(
            !eval
                .evaluate(&case("B"), &response("Answer: C"))
                .await
                .unwrap()
                .passed
        );
    }
}
