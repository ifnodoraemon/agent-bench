use crate::dataset::Category;
use crate::metrics::EloCalculator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DimensionScores {
    pub goal_score: Option<f64>,
    pub tool_score: Option<f64>,
    pub reasoning_score: Option<f64>,
    pub recovery_score: Option<f64>,
    pub efficiency_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub test_case_id: String,
    pub test_case_name: Option<String>,
    pub category: Category,
    pub passed: bool,
    pub score: f64,
    #[serde(default)]
    pub dimensions: Option<DimensionScores>,
    pub reason: String,
    pub latency_ms: u64,
    pub ttft_ms: Option<u64>,
    pub tps: f64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost_usd: f64,
    pub model_output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategorySummary {
    pub total_cases: usize,
    pub passed_cases: usize,
    pub accuracy: f64,
    pub avg_score: f64,
    pub avg_latency_ms: f64,
    pub avg_tps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBenchmarkSummary {
    pub model_id: String,
    pub model_name: String,
    pub total_cases: usize,
    pub passed_cases: usize,
    pub overall_accuracy: f64,          // Micro-Average Accuracy
    pub overall_score: f64,             // Micro-Average Soft Score
    pub macro_accuracy: f64,            // Macro-Average (Equal-weight per category)
    pub macro_score: f64,               // Macro-Average Soft Score
    pub weighted_composite_index: f64,  // Calibrated Difficulty Weighted Index (0-100)
    #[serde(default = "default_elo")]
    pub elo_rating: f64,                // Dynamic ELO Battle Rating
    #[serde(default)]
    pub efficiency_index: f64,          // Throughput & Latency Efficiency Score
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub avg_ttft_ms: Option<f64>,
    pub avg_tps: f64,
    pub total_prompt_tokens: u32,
    pub total_completion_tokens: u32,
    pub total_cost_usd: f64,
    pub category_summaries: HashMap<String, CategorySummary>,
    pub case_results: Vec<CaseResult>,
}

fn default_elo() -> f64 {
    1200.0
}

impl ModelBenchmarkSummary {
    pub fn compute(
        model_id: String,
        model_name: String,
        case_results: Vec<CaseResult>,
    ) -> Self {
        let total_cases = case_results.len();
        if total_cases == 0 {
            return Self {
                model_id,
                model_name,
                total_cases: 0,
                passed_cases: 0,
                overall_accuracy: 0.0,
                overall_score: 0.0,
                macro_accuracy: 0.0,
                macro_score: 0.0,
                weighted_composite_index: 0.0,
                elo_rating: 1200.0,
                efficiency_index: 0.0,
                avg_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                avg_ttft_ms: None,
                avg_tps: 0.0,
                total_prompt_tokens: 0,
                total_completion_tokens: 0,
                total_cost_usd: 0.0,
                category_summaries: HashMap::new(),
                case_results,
            };
        }

        let passed_cases = case_results.iter().filter(|c| c.passed).count();
        let overall_accuracy = passed_cases as f64 / total_cases as f64;
        let overall_score = case_results.iter().map(|c| c.score).sum::<f64>() / total_cases as f64;

        let total_prompt_tokens = case_results.iter().map(|c| c.prompt_tokens).sum();
        let total_completion_tokens = case_results.iter().map(|c| c.completion_tokens).sum();
        let total_cost_usd = case_results.iter().map(|c| c.cost_usd).sum();

        let avg_latency_ms = case_results.iter().map(|c| c.latency_ms as f64).sum::<f64>() / total_cases as f64;
        
        let mut latencies: Vec<u64> = case_results.iter().map(|c| c.latency_ms).collect();
        latencies.sort_unstable();
        let p95_idx = ((latencies.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
        let p95_latency_ms = latencies.get(p95_idx).cloned().unwrap_or(0) as f64;

        let ttft_values: Vec<u64> = case_results.iter().filter_map(|c| c.ttft_ms).collect();
        let avg_ttft_ms = if !ttft_values.is_empty() {
            Some(ttft_values.iter().sum::<u64>() as f64 / ttft_values.len() as f64)
        } else {
            None
        };

        let tps_values: Vec<f64> = case_results.iter().map(|c| c.tps).filter(|&t| t > 0.0).collect();
        let avg_tps = if !tps_values.is_empty() {
            tps_values.iter().sum::<f64>() / tps_values.len() as f64
        } else {
            0.0
        };

        // Category breakdown
        let mut cat_map: HashMap<String, Vec<&CaseResult>> = HashMap::new();
        for case in &case_results {
            cat_map.entry(case.category.as_str().to_string()).or_default().push(case);
        }

        let mut category_summaries = HashMap::new();
        for (cat_name, list) in cat_map {
            let cat_total = list.len();
            let cat_passed = list.iter().filter(|c| c.passed).count();
            let cat_avg_score = list.iter().map(|c| c.score).sum::<f64>() / cat_total as f64;
            let cat_avg_lat = list.iter().map(|c| c.latency_ms as f64).sum::<f64>() / cat_total as f64;
            let cat_tps_list: Vec<f64> = list.iter().map(|c| c.tps).filter(|&t| t > 0.0).collect();
            let cat_avg_tps = if !cat_tps_list.is_empty() {
                cat_tps_list.iter().sum::<f64>() / cat_tps_list.len() as f64
            } else {
                0.0
            };

            category_summaries.insert(
                cat_name,
                CategorySummary {
                    total_cases: cat_total,
                    passed_cases: cat_passed,
                    accuracy: cat_passed as f64 / cat_total as f64,
                    avg_score: cat_avg_score,
                    avg_latency_ms: cat_avg_lat,
                    avg_tps: cat_avg_tps,
                },
            );
        }

        // Compute Macro-Accuracy and Macro-Score (Equal weight across all categories)
        let num_cats = category_summaries.len();
        let (macro_accuracy, macro_score) = if num_cats > 0 {
            let sum_acc: f64 = category_summaries.values().map(|c| c.accuracy).sum();
            let sum_score: f64 = category_summaries.values().map(|c| c.avg_score).sum();
            (sum_acc / num_cats as f64, sum_score / num_cats as f64)
        } else {
            (overall_accuracy, overall_score)
        };

        // Calibrated Domain Weights for Composite Index
        let domain_weights: HashMap<&str, f64> = [
            ("swe", 0.15),
            ("agent", 0.15),
            ("devops", 0.10),
            ("math_logic", 0.10),
            ("security", 0.10),
            ("data_analyst", 0.08),
            ("medical", 0.06),
            ("legal", 0.06),
            ("finance", 0.06),
            ("science", 0.06),
            ("humanities", 0.04),
            ("safety", 0.04),
        ]
        .iter()
        .cloned()
        .collect();

        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;
        for (cat_name, summary) in &category_summaries {
            let w = domain_weights.get(cat_name.as_str()).cloned().unwrap_or(0.05);
            weighted_sum += summary.avg_score * w;
            weight_total += w;
        }

        let weighted_composite_index = if weight_total > 0.0 {
            (weighted_sum / weight_total) * 100.0
        } else {
            overall_score * 100.0
        };

        // Efficiency index: Throughput * Accuracy normalized by log(latency)
        let latency_factor = (avg_latency_ms / 1000.0).max(1.0).ln() + 1.0;
        let efficiency_index = (overall_accuracy * avg_tps / latency_factor).max(0.0);

        Self {
            model_id,
            model_name,
            total_cases,
            passed_cases,
            overall_accuracy,
            overall_score,
            macro_accuracy,
            macro_score,
            weighted_composite_index,
            elo_rating: 1200.0,
            efficiency_index,
            avg_latency_ms,
            p95_latency_ms,
            avg_ttft_ms,
            avg_tps,
            total_prompt_tokens,
            total_completion_tokens,
            total_cost_usd,
            category_summaries,
            case_results,
        }
    }

    /// Compute head-to-head Elo ratings across all evaluated models based on matching test cases
    pub fn compute_head_to_head_elo(summaries: &mut [ModelBenchmarkSummary]) {
        if summaries.len() < 2 {
            return;
        }

        let elo = EloCalculator::new(24.0, 1200.0);
        let n = summaries.len();

        for i in 0..n {
            for j in (i + 1)..n {
                // Map cases of model j by ID for fast lookup
                let map_b: HashMap<&str, &CaseResult> = summaries[j]
                    .case_results
                    .iter()
                    .map(|c| (c.test_case_id.as_str(), c))
                    .collect();

                let mut rating_a = summaries[i].elo_rating;
                let mut rating_b = summaries[j].elo_rating;

                for case_a in &summaries[i].case_results {
                    if let Some(case_b) = map_b.get(case_a.test_case_id.as_str()) {
                        let score_a = if (case_a.score - case_b.score).abs() < 0.05 {
                            0.5 // Tie
                        } else if case_a.score > case_b.score {
                            1.0 // Win A
                        } else {
                            0.0 // Win B
                        };

                        let (new_a, new_b) = elo.update_rating(rating_a, rating_b, score_a);
                        rating_a = new_a;
                        rating_b = new_b;
                    }
                }

                summaries[i].elo_rating = rating_a;
                summaries[j].elo_rating = rating_b;
            }
        }
    }
}
