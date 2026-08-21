use crate::metrics::ModelBenchmarkSummary;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, Table};

pub struct TerminalReporter;

impl TerminalReporter {
    pub fn render_summary(summaries: &[ModelBenchmarkSummary]) -> String {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Model").add_attribute(Attribute::Bold),
                Cell::new("Elo Rating").add_attribute(Attribute::Bold),
                Cell::new("Micro Acc").add_attribute(Attribute::Bold),
                Cell::new("Macro Acc").add_attribute(Attribute::Bold),
                Cell::new("L4/L5 Frontier").add_attribute(Attribute::Bold),
                Cell::new("Composite").add_attribute(Attribute::Bold),
                Cell::new("Avg Latency").add_attribute(Attribute::Bold),
                Cell::new("P95 Latency").add_attribute(Attribute::Bold),
                Cell::new("TTFT").add_attribute(Attribute::Bold),
                Cell::new("TPS").add_attribute(Attribute::Bold),
                Cell::new("Tokens (In/Out)").add_attribute(Attribute::Bold),
            ]);

        let mut sorted_summaries = summaries.to_vec();
        sorted_summaries.sort_by(|a, b| {
            b.overall_accuracy
                .partial_cmp(&a.overall_accuracy)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.elo_rating.partial_cmp(&a.elo_rating).unwrap_or(std::cmp::Ordering::Equal))
        });

        for summary in &sorted_summaries {
            let acc_color = if summary.overall_accuracy >= 0.85 {
                Color::Green
            } else if summary.overall_accuracy >= 0.60 {
                Color::Yellow
            } else {
                Color::Red
            };

            let frontier_color = if summary.l4_l5_frontier_accuracy >= 0.85 {
                Color::Green
            } else if summary.l4_l5_frontier_accuracy >= 0.60 {
                Color::Yellow
            } else {
                Color::Red
            };

            let ttft_str = summary
                .avg_ttft_ms
                .map(|t| format!("{:.0}ms", t))
                .unwrap_or_else(|| "-".to_string());

            table.add_row(vec![
                Cell::new(&summary.model_name).add_attribute(Attribute::Bold),
                Cell::new(format!("{:.0}", summary.elo_rating)).fg(Color::Cyan),
                Cell::new(format!(
                    "{:.1}% ({}/{})",
                    summary.overall_accuracy * 100.0,
                    summary.passed_cases,
                    summary.total_cases
                ))
                .fg(acc_color),
                Cell::new(format!("{:.1}%", summary.macro_accuracy * 100.0)).fg(acc_color),
                Cell::new(format!("{:.1}%", summary.l4_l5_frontier_accuracy * 100.0)).fg(frontier_color),
                Cell::new(format!("{:.1}", summary.weighted_composite_index)).fg(Color::Magenta),
                Cell::new(format!("{:.0}ms", summary.avg_latency_ms)),
                Cell::new(format!("{:.0}ms", summary.p95_latency_ms)),
                Cell::new(ttft_str),
                Cell::new(format!("{:.1}", summary.avg_tps)),
                Cell::new(format!(
                    "{}/{}",
                    summary.total_prompt_tokens, summary.total_completion_tokens
                )),
            ]);
        }

        let mut output = String::new();
        output.push_str("\n📊 === BENCHMARK EVALUATION LEADERBOARD ===\n\n");
        output.push_str(&table.to_string());
        output.push_str("\n\n");

        // Category breakdown table
        let mut cat_table = Table::new();
        cat_table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Model").add_attribute(Attribute::Bold),
                Cell::new("Category").add_attribute(Attribute::Bold),
                Cell::new("Passed / Total").add_attribute(Attribute::Bold),
                Cell::new("Accuracy").add_attribute(Attribute::Bold),
                Cell::new("Avg Score").add_attribute(Attribute::Bold),
                Cell::new("Avg Latency").add_attribute(Attribute::Bold),
            ]);

        for summary in summaries {
            let mut sorted_cats: Vec<_> = summary.category_summaries.iter().collect();
            sorted_cats.sort_by_key(|(k, _)| (*k).clone());
            for (cat, cat_stats) in sorted_cats {
                let acc_color = if cat_stats.accuracy >= 0.8 {
                    Color::Green
                } else if cat_stats.accuracy >= 0.5 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                cat_table.add_row(vec![
                    Cell::new(&summary.model_name),
                    Cell::new(cat).add_attribute(Attribute::Bold),
                    Cell::new(format!("{}/{}", cat_stats.passed_cases, cat_stats.total_cases)),
                    Cell::new(format!("{:.1}%", cat_stats.accuracy * 100.0)).fg(acc_color),
                    Cell::new(format!("{:.2}", cat_stats.avg_score)),
                    Cell::new(format!("{:.0}ms", cat_stats.avg_latency_ms)),
                ]);
            }
        }

        output.push_str("📁 === CATEGORY BREAKDOWN ===\n\n");
        output.push_str(&cat_table.to_string());
        output.push_str("\n");

        output
    }
}
