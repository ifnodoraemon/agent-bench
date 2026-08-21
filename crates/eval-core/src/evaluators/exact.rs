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

    /// Extract clean answer from potential Markdown/LaTeX wrappers like \boxed{...} or \frac{a}{b}
    pub fn normalize_answer(text: &str) -> String {
        let mut s = text.trim().to_string();

        // 1. Extract content inside \boxed{...} if present
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

        // 2. Normalize \frac{a}{b} -> a/b
        while let Some(frac_idx) = s.find(r"\frac{") {
            let after_frac = &s[frac_idx + 6..];
            if let Some(close_num) = after_frac.find('}') {
                let num = &after_frac[..close_num];
                let rest = &after_frac[close_num + 1..];
                if rest.starts_with('{') {
                    if let Some(close_den) = rest[1..].find('}') {
                        let den = &rest[1..1 + close_den];
                        let replacement = format!("{}/{}", num.trim(), den.trim());
                        let total_len = 6 + close_num + 1 + 1 + close_den + 1;
                        s.replace_range(frac_idx..frac_idx + total_len, &replacement);
                        continue;
                    }
                }
            }
            break;
        }

        // 3. Remove LaTeX wrappers like \[, \], \(, \), $$, $
        s = s.replace(r"\[", "")
            .replace(r"\]", "")
            .replace(r"\(", "")
            .replace(r"\)", "")
            .replace("$$", "")
            .replace('$', "");

        // 4. Strip trailing punctuation
        s = s.trim_end_matches(['.', ',', '!', '?', ';', ':', ' ']).trim().to_string();
        s
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

        let raw_output = response.text.trim();
        let target = reference.trim();

        let norm_output = Self::normalize_answer(raw_output);
        let norm_target = Self::normalize_answer(target);

        let (final_output, final_target) = if !self.case_sensitive {
            (norm_output.to_lowercase(), norm_target.to_lowercase())
        } else {
            (norm_output, norm_target)
        };

        if final_output == final_target {
            Ok(EvaluationResult::pass("Exact match succeeded"))
        } else if final_output.contains(&final_target) || raw_output.to_lowercase().contains(&final_target.to_lowercase()) {
            Ok(EvaluationResult::partial(
                0.8,
                format!("Output contains target reference answer (Matched: '{target}')"),
            ))
        } else {
            Ok(EvaluationResult::fail(format!(
                "Mismatch. Expected: '{target}', got: '{raw_output}'"
            )))
        }
    }
}

