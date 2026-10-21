use crate::config::ConfigFile;
use anyhow::{Context, Result};
use chrono::Local;
use eval_core::dataset::{Dataset, DatasetLoader};
use eval_core::metrics::ModelBenchmarkSummary;
use eval_core::model::create_client;
use eval_core::reporter::{HtmlReporter, MarkdownReporter, TerminalReporter};
use eval_suites::BenchmarkOrchestrator;
use std::fs;
use std::path::{Path, PathBuf};

fn collect_jsonl_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            if child_path.is_dir() {
                collect_jsonl_files(&child_path, files)?;
            } else if child_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                files.push(child_path);
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_run(
    config_path: Option<String>,
    dataset_paths: Vec<String>,
    filter_category: Option<String>,
    filter_difficulty: Option<String>,
    filter_frontier: bool,
    filter_eval_type: Option<String>,
    filter_tag: Option<String>,
    limit: Option<usize>,
    sample_ratio: Option<f64>,
    seed: Option<u64>,
    resume_failed: Option<String>,
    models_override: Option<Vec<String>>,
    concurrency_override: Option<usize>,
    output_dir_override: Option<String>,
    allow_mock: bool,
) -> Result<()> {
    // 1. Load config file if available, or use defaults
    let config = if let Some(path) = config_path {
        ConfigFile::load_from_file(path)?
    } else if Path::new("eval_config.toml").exists() {
        ConfigFile::load_from_file("eval_config.toml")?
    } else {
        ConfigFile::default()
    };

    let concurrency = concurrency_override.unwrap_or(config.benchmark.concurrency);
    let output_dir = output_dir_override.unwrap_or(config.benchmark.output_dir.clone());

    // 2. Resolve datasets
    let mut combined_dataset = Dataset::new("unified_benchmark");
    let target_paths = if !dataset_paths.is_empty() {
        dataset_paths
    } else if Path::new("datasets").is_dir() {
        vec!["datasets".to_string()]
    } else {
        // Look for default datasets folder
        let default_ds = vec![
            "datasets/foundation/knowledge_math.jsonl".to_string(),
            "datasets/foundation/code_generation.jsonl".to_string(),
            "datasets/foundation/instruction_following.jsonl".to_string(),
            "datasets/foundation/long_context.jsonl".to_string(),
            "datasets/foundation/structured_output.jsonl".to_string(),
            "datasets/agent/tool_calling.jsonl".to_string(),
            "datasets/agent/coding_tasks.jsonl".to_string(),
            "datasets/agent/error_recovery.jsonl".to_string(),
            "datasets/agent/multiturn_react.jsonl".to_string(),
            "datasets/safety/jailbreak_injection.jsonl".to_string(),
            "datasets/safety/hallucination.jsonl".to_string(),
            "datasets/safety/privacy_pii.jsonl".to_string(),
        ];
        default_ds
            .into_iter()
            .filter(|p| Path::new(p).exists())
            .collect()
    };

    if target_paths.is_empty() {
        println!("⚠️ No datasets found or specified! Please provide dataset file paths.");
        return Ok(());
    }

    let mut all_files = Vec::new();
    for path_str in &target_paths {
        let p = Path::new(path_str);
        if !p.exists() {
            println!("⚠️ Dataset path not found: {path_str}");
            continue;
        }
        collect_jsonl_files(p, &mut all_files)?;
    }

    if all_files.is_empty() {
        println!("⚠️ No .jsonl dataset files discovered in specified paths.");
        return Ok(());
    }

    all_files.sort();

    for file_path in &all_files {
        let ds = DatasetLoader::load_from_jsonl(file_path)?;
        combined_dataset.test_cases.extend(ds.test_cases);
    }

    // Apply filtering
    if let Some(cat_str) = filter_category {
        combined_dataset = combined_dataset.filter_by_group(&cat_str);
    }
    if let Some(ref tag) = filter_tag {
        combined_dataset = combined_dataset.filter_by_tag(tag);
    }
    if filter_frontier {
        combined_dataset = combined_dataset.filter_by_difficulty("frontier");
    }
    if let Some(ref diff) = filter_difficulty {
        combined_dataset = combined_dataset.filter_by_difficulty(diff);
    }
    if let Some(ref et_str) = filter_eval_type {
        if let Some(et) = eval_core::dataset::EvaluationType::from_str(et_str) {
            combined_dataset = combined_dataset.filter_by_eval_type(&et);
        } else {
            eprintln!("⚠️ Unknown evaluation type: '{et_str}'. Valid types: exact_match, regex, json_schema, code_execution, llm_judge, agent_trajectory");
        }
    }
    let mut resume_original: Vec<ModelBenchmarkSummary> = Vec::new();
    if let Some(ref resume_path) = resume_failed {
        let path = Path::new(resume_path);
        if !path.exists() {
            anyhow::bail!("Resume file not found: {resume_path}");
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read resume results file: {resume_path}"))?;
        let mut failed_ids = std::collections::HashSet::new();
        if let Ok(summaries) = serde_json::from_str::<Vec<ModelBenchmarkSummary>>(&content) {
            for s in &summaries {
                for c in &s.case_results {
                    if !c.passed || c.error.is_some() {
                        failed_ids.insert(c.test_case_id.clone());
                    }
                }
            }
            resume_original = summaries;
        } else if let Ok(single) = serde_json::from_str::<ModelBenchmarkSummary>(&content) {
            for c in &single.case_results {
                if !c.passed || c.error.is_some() {
                    failed_ids.insert(c.test_case_id.clone());
                }
            }
            resume_original.push(single);
        } else {
            anyhow::bail!("Resume file is not a valid benchmark result: {resume_path}");
        }
        println!(
            "🔁 Resuming: isolated {} failed/errored test cases from '{}'",
            failed_ids.len(),
            path.display()
        );
        combined_dataset = combined_dataset.filter_by_ids(&failed_ids);
    }
    if let Some(ratio) = sample_ratio {
        combined_dataset = combined_dataset.sample(ratio, seed);
    }
    if let Some(lim) = limit {
        combined_dataset = combined_dataset.limit(lim);
    }

    println!(
        "🎯 Loaded {} test cases across target datasets.",
        combined_dataset.test_cases.len()
    );

    if combined_dataset.test_cases.is_empty() {
        anyhow::bail!("Dataset is empty after filtering. Check --category/--difficulty/--tag/--resume filters.");
    }

    // 3. Resolve target models
    let mut model_profiles = config.models.clone();
    if let Some(overrides) = models_override {
        if !overrides.is_empty() {
            model_profiles
                .retain(|m| overrides.contains(&m.id) || overrides.contains(&m.model_name));
        }
    }

    // Never silently substitute mock models for a real benchmark.
    if model_profiles.is_empty() {
        if !allow_mock {
            anyhow::bail!(
                "No models configured. Copy eval_config.example.toml to eval_config.toml and fill in your endpoints, or pass --mock for a demo run."
            );
        }
        println!(
            "ℹ️ --mock: using built-in Mock models. These results are not real model measurements."
        );
        let mut mock1 = eval_core::model::ModelConfig::new("mock-pro", "mock", "Mock-Pro-v1");
        mock1.price_per_input_million = Some(0.5);
        mock1.price_per_output_million = Some(1.5);

        let mut mock2 = eval_core::model::ModelConfig::new("mock-fast", "mock", "Mock-Fast-v1");
        mock2.price_per_input_million = Some(0.1);
        mock2.price_per_output_million = Some(0.2);

        let c1 = eval_core::model::create_client(mock1)?;
        let c2 = eval_core::model::create_client(mock2)?;

        let orchestrator = BenchmarkOrchestrator::new(concurrency);
        let outcomes = orchestrator
            .run_all_models_parallel(vec![c1, c2], &combined_dataset)
            .await?;
        let (summaries, failures) = split_outcomes(outcomes);
        if summaries.is_empty() {
            anyhow::bail!("All mock model evaluations failed.");
        }
        let summaries = merge_resume_summaries(summaries, &resume_original);
        save_and_display_results(&summaries, &output_dir, &config.benchmark)?;
        if !failures.is_empty() {
            anyhow::bail!(
                "{} model(s) failed: {}",
                failures.len(),
                failures.join(", ")
            );
        }
        return Ok(());
    }

    // Optional judge client
    let judge_client = if let Some(judge_prof) = config.judge {
        Some(create_client(judge_prof.to_model_config())?)
    } else {
        None
    };

    let mut orchestrator = BenchmarkOrchestrator::new(concurrency);
    if let Some(judge) = judge_client {
        orchestrator = orchestrator.with_judge(judge);
    }

    let mut clients = Vec::new();
    for profile in &model_profiles {
        let model_cfg = profile.to_model_config();
        let client = create_client(model_cfg)
            .with_context(|| format!("Failed to initialize client for '{}'", profile.id))?;
        clients.push(client);
    }

    println!(
        "\n🚀 Launching parallel matrix benchmark on {} models concurrently...",
        clients.len()
    );
    for profile in &model_profiles {
        println!("  • [{}] {}", profile.id, profile.model_name);
    }
    println!();

    let outcomes = orchestrator
        .run_all_models_parallel(clients, &combined_dataset)
        .await?;
    let (summaries, failures) = split_outcomes(outcomes);
    if summaries.is_empty() {
        anyhow::bail!("All model evaluations failed.");
    }
    let summaries = merge_resume_summaries(summaries, &resume_original);

    // Persist successful models first; then report the failed models with a
    // non-zero exit so CI cannot mistake a partial matrix for a complete one.
    save_and_display_results(&summaries, &output_dir, &config.benchmark)?;
    if !failures.is_empty() {
        anyhow::bail!(
            "{} model(s) failed and were not written as complete results: {}",
            failures.len(),
            failures.join(", ")
        );
    }
    Ok(())
}

fn split_outcomes(
    outcomes: Vec<(String, anyhow::Result<ModelBenchmarkSummary>)>,
) -> (Vec<ModelBenchmarkSummary>, Vec<String>) {
    let mut summaries = Vec::new();
    let mut failures = Vec::new();
    for (model_id, outcome) in outcomes {
        match outcome {
            Ok(summary) => summaries.push(summary),
            Err(err) => {
                eprintln!("❌ Model '{model_id}' failed: {err}");
                failures.push(model_id);
            }
        }
    }
    (summaries, failures)
}

fn merge_resume_summaries(
    mut new_summaries: Vec<ModelBenchmarkSummary>,
    original: &[ModelBenchmarkSummary],
) -> Vec<ModelBenchmarkSummary> {
    if original.is_empty() {
        return new_summaries;
    }
    for original_summary in original {
        if let Some(new_summary) = new_summaries
            .iter_mut()
            .find(|s| s.model_id == original_summary.model_id)
        {
            let mut by_id: std::collections::HashMap<String, eval_core::metrics::CaseResult> =
                original_summary
                    .case_results
                    .iter()
                    .map(|c| (c.test_case_id.clone(), c.clone()))
                    .collect();
            for case in &new_summary.case_results {
                by_id.insert(case.test_case_id.clone(), case.clone());
            }
            let mut merged_cases: Vec<_> = original_summary
                .case_results
                .iter()
                .filter_map(|c| by_id.remove(&c.test_case_id))
                .collect();
            merged_cases.extend(by_id.into_values());
            *new_summary = ModelBenchmarkSummary::compute(
                new_summary.model_id.clone(),
                new_summary.model_name.clone(),
                merged_cases,
            );
        } else {
            new_summaries.push(original_summary.clone());
        }
    }
    new_summaries
}

fn save_and_display_results(
    summaries: &[ModelBenchmarkSummary],
    output_dir: &str,
    bench_config: &crate::config::BenchGlobalConfig,
) -> Result<()> {
    // 1. Terminal Table
    let table_str = TerminalReporter::render_summary(summaries);
    println!("{table_str}");

    // 2. Ensure output directory exists
    fs::create_dir_all(output_dir)?;
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

    // 3. Save JSON
    if bench_config.save_json {
        let json_path = PathBuf::from(output_dir).join(format!("eval_results_{timestamp}.json"));
        let json_str = serde_json::to_string_pretty(summaries)?;
        fs::write(&json_path, json_str)?;
        println!("💾 Saved raw JSON results to: {}", json_path.display());
    }

    // 4. Save Markdown
    if bench_config.save_markdown {
        let md_path = PathBuf::from(output_dir).join(format!("eval_report_{timestamp}.md"));
        let md_str = MarkdownReporter::generate_report(summaries);
        fs::write(&md_path, md_str)?;
        println!("📄 Saved Markdown report to: {}", md_path.display());
    }

    // 5. Save HTML
    if bench_config.save_html {
        let html_path = PathBuf::from(output_dir).join(format!("eval_dashboard_{timestamp}.html"));
        let html_str = HtmlReporter::generate_html(summaries);
        fs::write(&html_path, html_str)?;
        println!("🌐 Saved HTML dashboard to: {}", html_path.display());
    }

    Ok(())
}
