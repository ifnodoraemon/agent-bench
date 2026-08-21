use anyhow::{Context, Result};
use eval_core::metrics::{EloCalculator, ModelBenchmarkSummary};
use eval_core::reporter::TerminalReporter;
use std::fs;
use std::path::Path;

pub fn execute_compare(result_files: Vec<String>) -> Result<()> {
    if result_files.is_empty() {
        println!("⚠️ Please provide at least one result JSON file to compare.");
        return Ok(());
    }

    let mut all_summaries = Vec::new();

    for path_str in &result_files {
        let path = Path::new(path_str);
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read result file: {}", path.display()))?;

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

    // Render leaderboard table
    let table = TerminalReporter::render_summary(&all_summaries);
    println!("{table}");

    // Compute pairwise battle Elo if multiple models exist
    if all_summaries.len() >= 2 {
        println!("⚔️ === PAIRWISE ELO RATINGS ===");
        let elo_calc = EloCalculator::default();
        let model_names: Vec<String> = all_summaries.iter().map(|s| s.model_name.clone()).collect();
        let mut battles = Vec::new();

        // Compare score across identical test cases
        for i in 0..all_summaries.len() {
            for j in (i + 1)..all_summaries.len() {
                let m_a = &all_summaries[i];
                let m_b = &all_summaries[j];

                for case_a in &m_a.case_results {
                    if let Some(case_b) = m_b.case_results.iter().find(|c| c.test_case_id == case_a.test_case_id) {
                        let score_a = if case_a.score > case_b.score {
                            1.0
                        } else if (case_a.score - case_b.score).abs() < 0.001 {
                            0.5
                        } else {
                            0.0
                        };
                        battles.push((m_a.model_name.as_str(), m_b.model_name.as_str(), score_a));
                    }
                }
            }
        }

        let ratings = elo_calc.compute_ratings(&model_names, &battles);
        let mut sorted_ratings: Vec<_> = ratings.into_iter().collect();
        sorted_ratings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (rank, (model, elo)) in sorted_ratings.iter().enumerate() {
            println!("{}. {:<25} -> Elo: {:.0}", rank + 1, model, elo);
        }
        println!();
    }

    // Export modern light dashboard
    let html_content = eval_core::reporter::HtmlReporter::generate_html(&all_summaries);
    let out_dir = Path::new("./results");
    if !out_dir.exists() {
        let _ = fs::create_dir_all(out_dir);
    }
    let index_path = out_dir.join("index.html");
    fs::write(&index_path, html_content)
        .with_context(|| format!("Failed to write HTML dashboard to: {}", index_path.display()))?;
    println!("🌐 Visual dashboard updated at: {}", index_path.display());

    Ok(())
}
