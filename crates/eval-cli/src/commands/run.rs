use crate::config::ConfigFile;
use anyhow::{Context, Result};
use chrono::Local;
use eval_core::dataset::{Category, Dataset, DatasetLoader};
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

pub async fn execute_run(
    config_path: Option<String>,
    dataset_paths: Vec<String>,
    filter_category: Option<String>,
    filter_tag: Option<String>,
    models_override: Option<Vec<String>>,
    concurrency_override: Option<usize>,
    output_dir_override: Option<String>,
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
        let cat = match cat_str.to_lowercase().as_str() {
            "foundation" => Category::Foundation,
            "agent" => Category::Agent,
            "safety" => Category::Safety,
            "performance" => Category::Performance,
            other => Category::Custom(other.to_string()),
        };
        combined_dataset = combined_dataset.filter_by_category(&cat);
    }
    if let Some(ref tag) = filter_tag {
        combined_dataset = combined_dataset.filter_by_tag(tag);
    }

    println!(
        "🎯 Loaded {} test cases across target datasets.",
        combined_dataset.test_cases.len()
    );

    if combined_dataset.test_cases.is_empty() {
        println!("⚠️ Dataset is empty after filtering. Exiting.");
        return Ok(());
    }

    // 3. Resolve target models
    let mut model_profiles = config.models.clone();
    if let Some(overrides) = models_override {
        if !overrides.is_empty() {
            model_profiles.retain(|m| overrides.contains(&m.id) || overrides.contains(&m.model_name));
        }
    }

    // If no models defined in config, create a default mock model for demonstration
    if model_profiles.is_empty() {
        println!("ℹ️ No models configured in eval_config.toml. Using built-in Mock models for evaluation test.");
        let mut mock1 = eval_core::model::ModelConfig::new("mock-pro", "mock", "Mock-Pro-v1");
        mock1.price_per_input_million = Some(0.5);
        mock1.price_per_output_million = Some(1.5);

        let mut mock2 = eval_core::model::ModelConfig::new("mock-fast", "mock", "Mock-Fast-v1");
        mock2.price_per_input_million = Some(0.1);
        mock2.price_per_output_million = Some(0.2);

        let c1 = eval_core::model::create_client(mock1)?;
        let c2 = eval_core::model::create_client(mock2)?;

        let orchestrator = BenchmarkOrchestrator::new(concurrency);
        let summaries = orchestrator.run_all_models_parallel(vec![c1, c2], &combined_dataset).await?;

        save_and_display_results(&summaries, &output_dir, &config.benchmark)?;
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

    println!("\n🚀 Launching parallel matrix benchmark on {} models concurrently...", clients.len());
    for profile in &model_profiles {
        println!("  • [{}] {}", profile.id, profile.model_name);
    }
    println!();

    let summaries = orchestrator.run_all_models_parallel(clients, &combined_dataset).await?;

    save_and_display_results(&summaries, &output_dir, &config.benchmark)?;
    Ok(())
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
