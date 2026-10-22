use anyhow::{bail, Context, Result};
use chrono::Local;
use eval_core::dataset::{Dataset, DatasetLoader, EvaluationType, TestCase};
use eval_core::evaluators::{
    CodeSandboxEvaluator, Evaluator, ExactMatchEvaluator, F1Evaluator, IfEvalEvaluator,
    JsonSchemaEvaluator, MultipleChoiceEvaluator, NumericEvaluator, RegexEvaluator,
};
use eval_core::metrics::{CaseResult, ModelBenchmarkSummary};
use eval_core::model::{ModelResponse, TokenUsage};
use eval_core::reporter::{HtmlReporter, MarkdownReporter, TerminalReporter};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn collect_jsonl_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            if child.is_dir() {
                collect_jsonl_files(&child, files)?;
            } else if child.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                files.push(child);
            }
        }
    }
    Ok(())
}

fn load_cases(paths: &[String]) -> Result<HashMap<String, TestCase>> {
    let target_paths = if paths.is_empty() {
        vec!["datasets".to_string()]
    } else {
        paths.to_vec()
    };
    let mut files = Vec::new();
    for path in target_paths {
        collect_jsonl_files(Path::new(&path), &mut files)?;
    }
    if files.is_empty() {
        bail!("no dataset JSONL files found");
    }
    let mut map = HashMap::new();
    for file in files {
        let dataset: Dataset = DatasetLoader::load_from_jsonl(&file)
            .with_context(|| format!("failed to load {}", file.display()))?;
        for case in dataset.test_cases {
            map.insert(case.id.clone(), case);
        }
    }
    Ok(map)
}

fn is_deterministic(eval_type: &EvaluationType) -> bool {
    matches!(
        eval_type,
        EvaluationType::ExactMatch
            | EvaluationType::Regex
            | EvaluationType::JsonSchema
            | EvaluationType::CodeExecution
            | EvaluationType::Numeric
            | EvaluationType::MultipleChoice
            | EvaluationType::F1
            | EvaluationType::IfEval
    )
}

fn response_from_case(case: &CaseResult) -> ModelResponse {
    ModelResponse {
        text: case.model_output.clone(),
        reasoning_content: case.reasoning_content.clone(),
        // CaseResult does not persist tool_calls; tool_call cases are skipped.
        tool_calls: None,
        usage: TokenUsage {
            prompt_tokens: case.prompt_tokens,
            completion_tokens: case.completion_tokens,
            total_tokens: case.prompt_tokens.saturating_add(case.completion_tokens),
        },
        total_duration: Duration::from_millis(case.latency_ms),
        ttft: case.ttft_ms.map(Duration::from_millis),
        tokens_per_second: case.tps,
        estimated_cost_usd: case.cost_usd,
        finish_reason: None,
        raw_response: None,
    }
}

async fn rescore_case(case: &mut CaseResult, test_case: &TestCase) -> Result<()> {
    let response = response_from_case(case);
    let result = match test_case.eval_type {
        EvaluationType::ExactMatch => {
            ExactMatchEvaluator::default()
                .evaluate(test_case, &response)
                .await?
        }
        EvaluationType::Regex => RegexEvaluator.evaluate(test_case, &response).await?,
        EvaluationType::JsonSchema => JsonSchemaEvaluator.evaluate(test_case, &response).await?,
        EvaluationType::CodeExecution => {
            CodeSandboxEvaluator::default()
                .evaluate(test_case, &response)
                .await?
        }
        EvaluationType::Numeric => NumericEvaluator.evaluate(test_case, &response).await?,
        EvaluationType::MultipleChoice => {
            MultipleChoiceEvaluator
                .evaluate(test_case, &response)
                .await?
        }
        EvaluationType::F1 => F1Evaluator.evaluate(test_case, &response).await?,
        EvaluationType::IfEval => IfEvalEvaluator.evaluate(test_case, &response).await?,
        EvaluationType::LlmJudge | EvaluationType::AgentTrajectory | EvaluationType::ToolCall => {
            unreachable!("non-deterministic evaluator must be filtered before rescore")
        }
    };
    case.passed = result.passed;
    case.score = result.score;
    case.reason = result.reason;
    case.dimensions = result.dimensions;
    case.error = None;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_rescore(
    result_file: String,
    dataset_paths: Vec<String>,
    output_dir: Option<String>,
) -> Result<()> {
    let raw = fs::read_to_string(&result_file)
        .with_context(|| format!("failed to read result file: {result_file}"))?;
    let mut summaries: Vec<ModelBenchmarkSummary> = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse result JSON: {result_file}"))?;
    if summaries.is_empty() {
        bail!("result file contains no model summaries");
    }
    let cases = load_cases(&dataset_paths)?;

    let mut rescored = 0usize;
    let mut skipped = 0usize;
    let mut missing = 0usize;
    let mut errors = 0usize;

    for summary in &mut summaries {
        let old_provider = summary.provider.clone();
        let old_channel = summary.channel.clone();
        let old_canonical = summary.canonical_name.clone();
        let old_elo = summary.elo_rating;
        let mut updated_cases = summary.case_results.clone();
        for case in &mut updated_cases {
            let Some(test_case) = cases.get(&case.test_case_id) else {
                missing += 1;
                continue;
            };
            if !is_deterministic(&test_case.eval_type) {
                skipped += 1;
                continue;
            }
            match rescore_case(case, test_case).await {
                Ok(()) => rescored += 1,
                Err(e) => {
                    errors += 1;
                    case.passed = false;
                    case.score = 0.0;
                    case.reason = format!("[rescore error] {e}");
                    case.error = Some(e.to_string());
                }
            }
        }
        *summary = ModelBenchmarkSummary::compute(
            summary.model_id.clone(),
            summary.model_name.clone(),
            updated_cases,
        );
        summary.provider = old_provider;
        summary.channel = old_channel;
        summary.canonical_name = old_canonical;
        summary.elo_rating = old_elo;
    }

    let out = output_dir.unwrap_or_else(|| "results/rescored".to_string());
    fs::create_dir_all(&out)?;
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    println!("{}", TerminalReporter::render_summary(&summaries));
    let json_path = PathBuf::from(&out).join(format!("eval_results_{timestamp}.json"));
    fs::write(&json_path, serde_json::to_string_pretty(&summaries)?)?;
    let md_path = PathBuf::from(&out).join(format!("eval_report_{timestamp}.md"));
    fs::write(&md_path, MarkdownReporter::generate_report(&summaries))?;
    let html_path = PathBuf::from(&out).join(format!("eval_dashboard_{timestamp}.html"));
    fs::write(&html_path, HtmlReporter::generate_html(&summaries))?;
    println!(
        "♻️ Rescore complete: {rescored} rescored, {skipped} non-deterministic preserved, {missing} missing dataset cases, {errors} errors"
    );
    println!("💾 Saved JSON: {}", json_path.display());
    println!("📄 Saved Markdown: {}", md_path.display());
    println!("🌐 Saved HTML: {}", html_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eval_core::dataset::Category;

    fn f1_case() -> TestCase {
        TestCase {
            id: "f1_rescore".to_string(),
            name: None,
            category: Category::LongContext,
            difficulty: Some("L4".to_string()),
            tags: vec![],
            prompt: "q".to_string(),
            system_prompt: None,
            reference_answer: Some("Norman".to_string()),
            eval_type: EvaluationType::F1,
            criteria: None,
            schema: None,
            test_code: None,
            tools: None,
            max_turns: None,
            metadata: HashMap::new(),
        }
    }

    fn f1_result(passed: bool, reason: &str) -> CaseResult {
        CaseResult {
            test_case_id: "f1_rescore".to_string(),
            test_case_name: None,
            category: Category::LongContext,
            difficulty: Some("L4".to_string()),
            passed,
            score: if passed { 1.0 } else { 0.0 },
            dimensions: None,
            reason: reason.to_string(),
            latency_ms: 10,
            ttft_ms: None,
            tps: 0.0,
            prompt_tokens: 1,
            completion_tokens: 1,
            cost_usd: 0.0,
            model_output: "Norman".to_string(),
            reasoning_content: None,
            trajectory_steps: None,
            error: None,
        }
    }

    #[tokio::test]
    async fn rescored_case_updates_stale_f1_judgement() {
        let tc = f1_case();
        let mut old = f1_result(false, "Token F1 0.399 < 0.4");
        rescore_case(&mut old, &tc).await.unwrap();
        assert!(old.passed);
        assert!(old.reason.contains("Token F1"));
    }
}
