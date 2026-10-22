use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;

/// Token-level F1 evaluator for short-answer QA tasks such as LongBench.
/// Pass threshold is 0.4. A short answer whose tokens are all contained in a
/// longer reference also passes (precision-oriented containment), because
/// LongBench references are sometimes verbose while models answer with a span.
/// This remains an adaptation, not the official full metric.
#[derive(Default)]
pub struct F1Evaluator;

impl F1Evaluator {
    fn tokens(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect()
    }

    fn score(reference: &str, response: &str) -> f64 {
        let expected = Self::tokens(reference);
        let actual = Self::tokens(response);
        if expected.is_empty() || actual.is_empty() {
            return 0.0;
        }
        let mut remaining = expected.clone();
        let mut overlap = 0usize;
        for token in &actual {
            if let Some(pos) = remaining.iter().position(|t| t == token) {
                remaining.remove(pos);
                overlap += 1;
            }
        }
        let precision = overlap as f64 / actual.len() as f64;
        let recall = overlap as f64 / expected.len() as f64;
        if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        }
    }
}

#[async_trait]
impl Evaluator for F1Evaluator {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
        let Some(reference) = test_case.reference_answer.as_deref() else {
            return Ok(EvaluationResult::fail(
                "F1 evaluator requires reference_answer",
            ));
        };
        let score = Self::score(reference, &response.text);
        let rounded = (score * 1000.0).round() / 1000.0;
        let expected = Self::tokens(reference);
        let actual = Self::tokens(&response.text);
        let contained = !actual.is_empty()
            && !expected.is_empty()
            && actual.iter().all(|token| expected.contains(token));
        // Compare the rounded value: token-F1 can be 0.39999999999999997 for
        // a mathematically exact 0.4 boundary, which must pass.
        if rounded >= 0.4 {
            let mut result = EvaluationResult::pass(format!(
                "Token F1 {rounded:.3} >= 0.4 (reference: {reference})"
            ));
            result.score = rounded;
            Ok(result)
        } else if contained {
            let mut result = EvaluationResult::pass(format!(
                "Short answer contained in reference (token F1 {rounded:.3}; reference: {reference})"
            ));
            result.score = rounded.max(0.5);
            Ok(result)
        } else {
            Ok(EvaluationResult::fail(format!(
                "Token F1 {rounded:.3} < 0.4 and answer is not contained in reference (reference: {reference})"
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
            id: "f1".to_string(),
            name: None,
            category: Category::LongContext,
            difficulty: None,
            tags: vec![],
            prompt: "q".to_string(),
            system_prompt: None,
            reference_answer: Some(reference.to_string()),
            eval_type: EvaluationType::F1,
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
    async fn accepts_short_answer_overlap() {
        let res = F1Evaluator
            .evaluate(&case("Dracula"), &response("The answer is Dracula."))
            .await
            .unwrap();
        assert!(res.passed, "{}", res.reason);
        assert!(res.score >= 0.4);
    }

    #[tokio::test]
    async fn rejects_unrelated_answer() {
        let res = F1Evaluator
            .evaluate(&case("Dracula"), &response("Frankenstein"))
            .await
            .unwrap();
        assert!(!res.passed);
    }

    #[tokio::test]
    async fn accepts_exact_f1_threshold_at_rounding_boundary() {
        // This pair produces raw F1 0.39999999999999997, equivalent to 0.4.
        let reference = "The sides of the fuselage are sloped to create a conical section when the fuselage is formed.";
        let output = "Because tumbled-home sides make the fuselage section conical, causing the top longeron to form an elliptical “banana” shape unless the side panel is developed to true dimensions.";
        let res = F1Evaluator
            .evaluate(&case(reference), &response(output))
            .await
            .unwrap();
        assert!(res.passed, "{}", res.reason);
        assert!(res.score >= 0.4);
    }

    #[tokio::test]
    async fn accepts_short_answer_contained_in_long_reference() {
        let res = F1Evaluator
            .evaluate(
                &case("Himalchuli has three main peaks: East, West and North"),
                &response("Himalchuli"),
            )
            .await
            .unwrap();
        assert!(res.passed, "{}", res.reason);
        let bad = F1Evaluator
            .evaluate(
                &case("on the Cumberland Plain"),
                &response("Sydney, Australia"),
            )
            .await
            .unwrap();
        assert!(!bad.passed);
    }
}
