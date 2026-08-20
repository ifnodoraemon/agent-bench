use crate::dataset::Category;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub test_case_id: String,
    pub test_case_name: Option<String>,
    pub category: Category,
    pub passed: bool,
    pub score: f64,
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
    pub overall_accuracy: f64,
    pub overall_score: f64,
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

        Self {
            model_id,
            model_name,
            total_cases,
            passed_cases,
            overall_accuracy,
            overall_score,
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
}
