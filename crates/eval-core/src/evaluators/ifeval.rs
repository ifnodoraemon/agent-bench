use super::{EvaluationResult, Evaluator};
use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

static BULLET_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*[*-]\s+").unwrap());
static HIGHLIGHT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*[^*\n]+\*").unwrap());
static TITLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<[^<>]+>>").unwrap());
static PLACEHOLDER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[^\[\]]+\]").unwrap());

/// Deterministic IFEval subset evaluator. It covers 21 instruction types; the
/// dataset builder only keeps cases whose instructions are all in this set.
/// This is a reproducible subset, not the full official 25-type verifier.
#[derive(Default)]
pub struct IfEvalEvaluator;

impl IfEvalEvaluator {
    fn relation_ok(actual: i64, relation: &str, target: i64) -> bool {
        match relation {
            "at least" => actual >= target,
            "less than" => actual < target,
            "more than" => actual > target,
            "at most" => actual <= target,
            "equal" | "exactly" => actual == target,
            _ => false,
        }
    }

    fn kw_i64(kw: &Map<String, Value>, key: &str) -> Option<i64> {
        kw.get(key).and_then(Value::as_i64)
    }

    fn kw_str<'a>(kw: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
        kw.get(key).and_then(Value::as_str)
    }

    fn kw_strings(kw: &Map<String, Value>, key: &str) -> Vec<String> {
        kw.get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn check(
        instruction: &str,
        kw: &Map<String, Value>,
        prompt: &str,
        text: &str,
    ) -> std::result::Result<(), String> {
        let lower = text.to_lowercase();
        match instruction {
            "punctuation:no_comma" => {
                if text.contains(',') {
                    return Err("response contains a comma".to_string());
                }
            }
            "length_constraints:number_words" => {
                let target =
                    Self::kw_i64(kw, "num_words").ok_or_else(|| "missing num_words".to_string())?;
                let relation = Self::kw_str(kw, "relation").unwrap_or("at least");
                let actual = text.split_whitespace().count() as i64;
                if !Self::relation_ok(actual, relation, target) {
                    return Err(format!(
                        "word count {actual} does not satisfy {relation} {target}"
                    ));
                }
            }
            "length_constraints:number_sentences" => {
                let target = Self::kw_i64(kw, "num_sentences")
                    .ok_or_else(|| "missing num_sentences".to_string())?;
                let relation = Self::kw_str(kw, "relation").unwrap_or("at least");
                let actual = text
                    .split(['.', '!', '?'])
                    .filter(|s| !s.trim().is_empty())
                    .count() as i64;
                if !Self::relation_ok(actual, relation, target) {
                    return Err(format!(
                        "sentence count {actual} does not satisfy {relation} {target}"
                    ));
                }
            }
            "length_constraints:number_paragraphs" => {
                let target = Self::kw_i64(kw, "num_paragraphs")
                    .ok_or_else(|| "missing num_paragraphs".to_string())?;
                let actual = if text.contains("***") {
                    text.split("***").filter(|s| !s.trim().is_empty()).count() as i64
                } else {
                    text.split("\n\n").filter(|s| !s.trim().is_empty()).count() as i64
                };
                if actual != target {
                    return Err(format!("paragraph count {actual} != {target}"));
                }
            }
            "detectable_format:number_bullet_lists" => {
                let target = Self::kw_i64(kw, "num_bullets")
                    .ok_or_else(|| "missing num_bullets".to_string())?;
                let actual = BULLET_RE.find_iter(text).count() as i64;
                if actual != target {
                    return Err(format!("bullet count {actual} != {target}"));
                }
            }
            "detectable_format:number_highlighted_sections" => {
                let target = Self::kw_i64(kw, "num_highlights")
                    .ok_or_else(|| "missing num_highlights".to_string())?;
                let actual = HIGHLIGHT_RE.find_iter(text).count() as i64;
                if actual < target {
                    return Err(format!("highlight count {actual} < {target}"));
                }
            }
            "detectable_format:json_format" => {
                let trimmed = text.trim();
                let lower = trimmed.to_ascii_lowercase();
                let body_start = if lower.starts_with("```json") {
                    7
                } else if lower.starts_with("```") {
                    3
                } else {
                    0
                };
                let mut candidate = &trimmed[body_start..];
                if candidate.trim_end().ends_with("```") {
                    let end = candidate.trim_end().len() - 3;
                    candidate = &candidate[..end];
                }
                if serde_json::from_str::<Value>(candidate.trim()).is_err() {
                    return Err("response is not valid JSON".to_string());
                }
            }
            "detectable_format:title" => {
                if !TITLE_RE.is_match(text) {
                    return Err("missing <<title>> section".to_string());
                }
            }
            "detectable_format:multiple_sections" => {
                let splitter = Self::kw_str(kw, "section_spliter")
                    .ok_or_else(|| "missing section_spliter".to_string())?;
                let target = Self::kw_i64(kw, "num_sections")
                    .ok_or_else(|| "missing num_sections".to_string())?;
                let actual = text.matches(splitter).count() as i64;
                if actual < target {
                    return Err(format!("section count {actual} < {target}"));
                }
            }
            "change_case:english_lowercase" => {
                if text.chars().any(|c| c.is_ascii_uppercase()) {
                    return Err("response contains uppercase letters".to_string());
                }
            }
            "change_case:english_capital" => {
                if text.chars().any(|c| c.is_ascii_lowercase()) {
                    return Err("response contains lowercase letters".to_string());
                }
            }
            "startend:end_checker" => {
                let phrase = Self::kw_str(kw, "end_phrase")
                    .ok_or_else(|| "missing end_phrase".to_string())?;
                if !text.trim_end().ends_with(phrase) {
                    return Err(format!("response does not end with '{phrase}'"));
                }
            }
            "startend:quotation" => {
                let t = text.trim();
                let quoted = (t.starts_with('"') && t.ends_with('"'))
                    || (t.starts_with('“') && t.ends_with('”'))
                    || (t.starts_with('\'') && t.ends_with('\''));
                if !quoted {
                    return Err("response is not wrapped in quotes".to_string());
                }
            }
            "keywords:existence" => {
                for keyword in Self::kw_strings(kw, "keywords") {
                    if !lower.contains(&keyword.to_lowercase()) {
                        return Err(format!("missing keyword '{keyword}'"));
                    }
                }
            }
            "keywords:forbidden_words" => {
                for word in Self::kw_strings(kw, "forbidden_words") {
                    if lower.contains(&word.to_lowercase()) {
                        return Err(format!("contains forbidden word '{word}'"));
                    }
                }
            }
            "keywords:frequency" => {
                let keyword = Self::kw_str(kw, "keyword")
                    .ok_or_else(|| "missing keyword".to_string())?
                    .to_lowercase();
                let target =
                    Self::kw_i64(kw, "frequency").ok_or_else(|| "missing frequency".to_string())?;
                let relation = Self::kw_str(kw, "relation").unwrap_or("at least");
                let actual = lower.matches(&keyword).count() as i64;
                if !Self::relation_ok(actual, relation, target) {
                    return Err(format!(
                        "keyword frequency {actual} does not satisfy {relation} {target}"
                    ));
                }
            }
            "keywords:letter_frequency" => {
                let letter =
                    Self::kw_str(kw, "letter").ok_or_else(|| "missing letter".to_string())?;
                let target = Self::kw_i64(kw, "let_frequency")
                    .ok_or_else(|| "missing let_frequency".to_string())?;
                let relation = Self::kw_str(kw, "let_relation").unwrap_or("at least");
                let actual = text.matches(letter).count() as i64;
                if !Self::relation_ok(actual, relation, target) {
                    return Err(format!(
                        "letter frequency {actual} does not satisfy {relation} {target}"
                    ));
                }
            }
            "combination:repeat_prompt" => {
                let repeat = Self::kw_str(kw, "prompt_to_repeat")
                    .ok_or_else(|| "missing prompt_to_repeat".to_string())?;
                let compact_response = text.split_whitespace().collect::<Vec<_>>().join(" ");
                let compact_prompt = repeat.split_whitespace().collect::<Vec<_>>().join(" ");
                if !compact_response.contains(&compact_prompt) {
                    return Err("response does not repeat the prompt".to_string());
                }
            }
            "combination:two_responses" => {
                let parts: Vec<&str> = text.split("******").collect();
                if parts.len() != 2 || parts.iter().any(|p| p.trim().is_empty()) {
                    return Err("expected exactly two responses separated by ******".to_string());
                }
            }
            "detectable_content:number_placeholders" => {
                let target = Self::kw_i64(kw, "num_placeholders")
                    .ok_or_else(|| "missing num_placeholders".to_string())?;
                let actual = PLACEHOLDER_RE.find_iter(text).count() as i64;
                if actual < target {
                    return Err(format!("placeholder count {actual} < {target}"));
                }
            }
            "detectable_content:postscript" => {
                let marker = Self::kw_str(kw, "postscript_marker")
                    .ok_or_else(|| "missing postscript_marker".to_string())?;
                if !text.trim_end().ends_with(marker) {
                    return Err(format!("response does not end with postscript '{marker}'"));
                }
            }
            other => return Err(format!("unsupported IFEval instruction '{other}'")),
        }
        let _ = prompt;
        Ok(())
    }
}

#[async_trait]
impl Evaluator for IfEvalEvaluator {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult> {
        let Some(ids) = test_case
            .metadata
            .get("instruction_id_list")
            .and_then(Value::as_array)
        else {
            return Ok(EvaluationResult::fail(
                "ifeval requires metadata.instruction_id_list",
            ));
        };
        let kwargs = test_case
            .metadata
            .get("kwargs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut failures = Vec::new();
        for (i, id) in ids.iter().enumerate() {
            let Some(id) = id.as_str() else {
                failures.push("non-string instruction id".to_string());
                continue;
            };
            let empty = Map::new();
            let kw = kwargs.get(i).and_then(Value::as_object).unwrap_or(&empty);
            if let Err(e) = Self::check(id, kw, &test_case.prompt, &response.text) {
                failures.push(format!("{id}: {e}"));
            }
        }
        if failures.is_empty() {
            Ok(EvaluationResult::pass(format!(
                "All {} IFEval instructions satisfied",
                ids.len()
            )))
        } else {
            Ok(EvaluationResult::fail(format!(
                "IFEval failures: {}",
                failures.join("; ")
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{Category, EvaluationType};
    use crate::model::TokenUsage;
    use serde_json::json;
    use std::collections::HashMap;
    use std::time::Duration;

    fn case(ids: Vec<&str>, kwargs: Vec<Value>) -> TestCase {
        let mut metadata = HashMap::new();
        metadata.insert("instruction_id_list".to_string(), json!(ids));
        metadata.insert("kwargs".to_string(), json!(kwargs));
        TestCase {
            id: "ifeval".to_string(),
            name: None,
            category: Category::Instruction,
            difficulty: None,
            tags: vec![],
            prompt: "Write a response.".to_string(),
            system_prompt: None,
            reference_answer: None,
            eval_type: EvaluationType::IfEval,
            criteria: None,
            schema: None,
            test_code: None,
            tools: None,
            max_turns: None,
            metadata,
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
    async fn enforces_no_comma_and_keywords() {
        let kw = json!({"keywords": ["alpha", "beta"]});
        let tc = case(
            vec!["punctuation:no_comma", "keywords:existence"],
            vec![json!({}), kw],
        );
        let ok = IfEvalEvaluator
            .evaluate(&tc, &response("alpha and beta"))
            .await
            .unwrap();
        assert!(ok.passed, "{}", ok.reason);
        let bad = IfEvalEvaluator
            .evaluate(&tc, &response("alpha, beta"))
            .await
            .unwrap();
        assert!(!bad.passed);
    }

    #[tokio::test]
    async fn enforces_format_rules() {
        let tc = case(
            vec!["detectable_format:json_format", "keywords:existence"],
            vec![json!({}), json!({"keywords": ["alpha"]})],
        );
        assert!(
            IfEvalEvaluator
                .evaluate(&tc, &response("{\"alpha\": 1}"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            !IfEvalEvaluator
                .evaluate(&tc, &response("not json alpha"))
                .await
                .unwrap()
                .passed
        );
    }

    #[tokio::test]
    async fn enforces_ending_rule() {
        let tc = case(
            vec!["startend:end_checker"],
            vec![json!({"end_phrase": "THE END"})],
        );
        assert!(
            IfEvalEvaluator
                .evaluate(&tc, &response("alpha THE END"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            !IfEvalEvaluator
                .evaluate(&tc, &response("alpha"))
                .await
                .unwrap()
                .passed
        );
    }

    #[tokio::test]
    async fn enforces_word_count_relation() {
        let tc = case(
            vec!["length_constraints:number_words"],
            vec![json!({"num_words": 3, "relation": "at least"})],
        );
        assert!(
            IfEvalEvaluator
                .evaluate(&tc, &response("one two three"))
                .await
                .unwrap()
                .passed
        );
        assert!(
            !IfEvalEvaluator
                .evaluate(&tc, &response("one two"))
                .await
                .unwrap()
                .passed
        );
    }
}
