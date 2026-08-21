use crate::metrics::ModelBenchmarkSummary;
use chrono::Local;

pub struct MarkdownReporter;

impl MarkdownReporter {
    pub fn generate_report(summaries: &[ModelBenchmarkSummary]) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut md = String::new();

        md.push_str(&format!("# 🏆 LLM & Agent Benchmark Report\n\n"));
        md.push_str(&format!("*Generated at: {}*\n\n", now));

        md.push_str("## 1. Overall Leaderboard\n\n");
        md.push_str("| Model | Accuracy | Avg Score | Avg Latency | P95 Latency | TTFT | TPS | In/Out Tokens | Total Cost |\n");
        md.push_str("| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |\n");

        for s in summaries {
            let ttft_str = s
                .avg_ttft_ms
                .map(|t| format!("{:.0}ms", t))
                .unwrap_or_else(|| "-".to_string());

            md.push_str(&format!(
                "| **{}** | {:.1}% ({}/{}) | {:.2} | {:.0}ms | {:.0}ms | {} | {:.1} | {}/{} | ${:.5} |\n",
                s.model_name,
                s.overall_accuracy * 100.0,
                s.passed_cases,
                s.total_cases,
                s.overall_score,
                s.avg_latency_ms,
                s.p95_latency_ms,
                ttft_str,
                s.avg_tps,
                s.total_prompt_tokens,
                s.total_completion_tokens,
                s.total_cost_usd
            ));
        }

        md.push_str("\n## 2. Category Performance\n\n");
        md.push_str("| Model | Category | Passed / Total | Accuracy | Avg Score | Avg Latency |\n");
        md.push_str("| :--- | :--- | :--- | :--- | :--- | :--- |\n");

        for s in summaries {
            let mut sorted_cats: Vec<_> = s.category_summaries.iter().collect();
            sorted_cats.sort_by_key(|(k, _)| (*k).clone());
            for (cat, stats) in sorted_cats {
                md.push_str(&format!(
                    "| {} | **{}** | {}/{} | {:.1}% | {:.2} | {:.0}ms |\n",
                    s.model_name,
                    cat,
                    stats.passed_cases,
                    stats.total_cases,
                    stats.accuracy * 100.0,
                    stats.avg_score,
                    stats.avg_latency_ms
                ));
            }
        }

        md.push_str("\n## 3. Failed Cases & Bad Cases Analysis\n\n");
        for s in summaries {
            let failed_cases: Vec<_> = s.case_results.iter().filter(|c| !c.passed).collect();
            if failed_cases.is_empty() {
                md.push_str(&format!("### Model: `{}` (All test cases passed! 🎉)\n\n", s.model_name));
            } else {
                md.push_str(&format!("### Model: `{}` (Failed: {} cases)\n\n", s.model_name, failed_cases.len()));
                for fc in failed_cases {
                    md.push_str(&format!("- **Case ID:** `{}` (Category: `{}`)\n", fc.test_case_id, fc.category.as_str()));
                    md.push_str(&format!("  - **Failure Reason:** {}\n", fc.reason));
                    let flat_output = fc.model_output.replace('\n', " ");
                    let char_count = flat_output.chars().count();
                    let snippet = if char_count > 150 {
                        format!("{}...", flat_output.chars().take(150).collect::<String>())
                    } else {
                        flat_output
                    };
                    md.push_str(&format!("  - **Model Output Snippet:** `{}`\n\n", snippet));
                }
            }
        }

        md
    }
}
