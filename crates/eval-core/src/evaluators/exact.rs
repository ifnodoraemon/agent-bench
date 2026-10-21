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

    /// Extract clean answer from potential Markdown/LaTeX wrappers like \boxed{...} or \frac{a}{b}.
    pub fn normalize_answer(text: &str) -> String {
        let mut s = text.trim().to_string();

        if let Some(boxed_idx) = s.rfind(r"\boxed{") {
            let inner = &s[boxed_idx + 7..];
            let mut depth = 1;
            let mut end_idx = 0;
            for (idx, ch) in inner.char_indices() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = idx;
                        break;
                    }
                }
            }
            if end_idx > 0 {
                s = inner[..end_idx].to_string();
            }
        }

        while let Some(frac_idx) = s.find(r"\frac{") {
            let after_frac = &s[frac_idx + 6..];
            if let Some(close_num) = after_frac.find('}') {
                let num = &after_frac[..close_num];
                let rest = &after_frac[close_num + 1..];
                if let Some(stripped) = rest.strip_prefix('{') {
                    if let Some(close_den) = stripped.find('}') {
                        let den = &stripped[..close_den];
                        let replacement = format!("{}/{}", num.trim(), den.trim());
                        let total_len = 6 + close_num + 1 + 1 + close_den + 1;
                        s.replace_range(frac_idx..frac_idx + total_len, &replacement);
                        continue;
                    }
                }
            }
            break;
        }

        s = s
            .replace(r"\[", "")
            .replace(r"\]", "")
            .replace(r"\(", "")
            .replace(r"\)", "")
            .replace("$$", "")
            .replace('$', "");
        s = s
            .trim_end_matches(['.', ',', '!', '?', ';', ':', ' '])
            .trim()
            .to_string();
        s
    }

    fn parse_number(s: &str) -> Option<f64> {
        let cleaned = s.trim().replace(',', "");
        cleaned.parse::<f64>().ok().filter(|v| v.is_finite())
    }
}

#[async_trait]
impl Evaluator for ExactMatchEvaluator {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
        let reference = match &test_case.reference_answer {
            Some(reference) => reference,
            None => {
                return Ok(EvaluationResult::fail(
                    "Missing reference_answer in test case",
                ))
            }
        };

        let raw_output = response.text.trim();
        let target = reference.trim();
        if target.is_empty() {
            return Ok(EvaluationResult::fail(
                "Empty reference_answer in test case",
            ));
        }

        let norm_output = Self::normalize_answer(raw_output);
        let norm_target = Self::normalize_answer(target);

        let (final_output, final_target) = if self.case_sensitive {
            (norm_output, norm_target)
        } else {
            (norm_output.to_lowercase(), norm_target.to_lowercase())
        };

        if final_output == final_target {
            return Ok(EvaluationResult::pass("Exact match succeeded"));
        }

        // Numeric equivalence is allowed only when both sides are a single
        // number: 4 == 4.0. Substring matching is deliberately not allowed,
        // because "12" must not match "2012" and "5" must not match "5.5".
        if let (Some(a), Some(b)) = (
            Self::parse_number(&final_output),
            Self::parse_number(&final_target),
        ) {
            let tolerance = 1e-9 * a.abs().max(b.abs()).max(1.0);
            if (a - b).abs() <= tolerance {
                return Ok(EvaluationResult::pass(
                    "Numerically equivalent to reference answer",
                ));
            }
        }

        Ok(EvaluationResult::fail(format!(
            "Mismatch. Expected: '{target}', got: '{raw_output}'"
        )))
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
            id: "t".to_string(),
            name: None,
            category: Category::Foundation,
            difficulty: None,
            tags: vec![],
            prompt: "p".to_string(),
            system_prompt: None,
            reference_answer: Some(reference.to_string()),
            eval_type: EvaluationType::ExactMatch,
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
    async fn exact_and_numeric() {
        let evaluator = ExactMatchEvaluator::default();
        assert!(
            evaluator
                .evaluate(&case("Paris"), &response("paris"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            evaluator
                .evaluate(&case("4"), &response("4.0"))
                .await
                .unwrap()
                .passed
        );
    }

    #[tokio::test]
    async fn rejects_substring_false_positives() {
        let evaluator = ExactMatchEvaluator::default();
        assert!(
            !evaluator
                .evaluate(&case("12"), &response("2012"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            !evaluator
                .evaluate(&case("5"), &response("5.5"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            !evaluator
                .evaluate(&case("Paris"), &response("The capital is Paris."))
                .await
                .unwrap()
                .passed
        );
    }

    #[tokio::test]
    async fn latex_box_is_supported() {
        let evaluator = ExactMatchEvaluator::default();
        let resp =
            response(r"probability = \(\frac{6}{36} = \frac{1}{6}\). \[ \boxed{\frac{1}{6}} \]");
        let res = evaluator.evaluate(&case("1/6"), &resp).await.unwrap();
        assert!(res.passed, "{}", res.reason);
    }
}
