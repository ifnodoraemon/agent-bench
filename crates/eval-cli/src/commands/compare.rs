use anyhow::{Context, Result};
use eval_core::metrics::ModelBenchmarkSummary;
use eval_core::reporter::TerminalReporter;
use std::fs;
use std::path::Path;

pub fn execute_compare(result_files: Vec<String>) -> Result<()> {
    if result_files.is_empty() {
        println!("⚠️ Please provide at least one result JSON file to compare.");
        return Ok(());
    }

    let mut resolved_paths = Vec::new();
    for path_str in &result_files {
        let path = Path::new(path_str);
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("json") {
                        resolved_paths.push(p);
                    }
                }
            }
        } else if path.exists() {
            resolved_paths.push(path.to_path_buf());
        }
    }

    if resolved_paths.is_empty() {
        println!("⚠️ No .json result files discovered in specified paths.");
        return Ok(());
    }

    resolved_paths.sort();
    let mut all_summaries = Vec::new();

    for path in &resolved_paths {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                println!("⚠️ Failed to read {}: {e}", path.display());
                continue;
            }
        };

        if let Ok(list) = serde_json::from_str::<Vec<ModelBenchmarkSummary>>(&content) {
            all_summaries.extend(list);
        } else if let Ok(single) = serde_json::from_str::<ModelBenchmarkSummary>(&content) {
            all_summaries.push(single);
        } else {
            println!("⚠️ Skipping unparseable result file: {}", path.display());
        }
    }

    if all_summaries.is_empty() {
        println!("⚠️ No valid benchmark summaries found.");
        return Ok(());
    }

    // Deduplicate by model_id to keep latest benchmark run per model
    let mut model_map = std::collections::BTreeMap::new();
    for s in all_summaries {
        model_map.insert(s.model_id.clone(), s);
    }
    let mut unique_summaries: Vec<ModelBenchmarkSummary> = model_map.into_values().collect();
    ModelBenchmarkSummary::compute_head_to_head_elo(&mut unique_summaries);

    // Render leaderboard table (Elo already converged).
    let table = TerminalReporter::render_summary(&unique_summaries);
    println!("{table}");

    if unique_summaries.len() >= 2 {
        println!("⚔️ === PAIRWISE ELO RATINGS (aggregated win-rate, heuristic) ===");
        let mut sorted: Vec<_> = unique_summaries
            .iter()
            .map(|s| (s.model_name.clone(), s.elo_rating))
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (rank, (model, elo)) in sorted.iter().enumerate() {
            println!("{}. {:<25} -> Elo: {:.0}", rank + 1, model, elo);
        }
        println!();
    }

    // Export modern light dashboard
    let html_content = eval_core::reporter::HtmlReporter::generate_html(&unique_summaries);
    let out_dir = Path::new("./results");
    if !out_dir.exists() {
        let _ = fs::create_dir_all(out_dir);
    }
    let index_path = out_dir.join("index.html");
    fs::write(&index_path, html_content).with_context(|| {
        format!(
            "Failed to write HTML dashboard to: {}",
            index_path.display()
        )
    })?;
    println!("🌐 Visual dashboard updated at: {}", index_path.display());

    Ok(())
}
