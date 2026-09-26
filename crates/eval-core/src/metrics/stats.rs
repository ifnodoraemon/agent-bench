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
    #[serde(default)]
    pub difficulty: Option<String>,
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
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub trajectory_steps: Option<Vec<serde_json::Value>>,
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
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub canonical_name: Option<String>,
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
    #[serde(default)]
    pub tier_accuracy: HashMap<String, f64>, // L1 ~ L5 Tier Accuracy Breakdown
    #[serde(default)]
    pub l4_l5_frontier_accuracy: f64,   // Extreme/Frontier Difficulty Accuracy (L4 + L5)
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

struct ChannelRule {
    channel: &'static str,
    keywords: &'static [&'static str],
}

static CHANNEL_RULES: &[ChannelRule] = &[
    ChannelRule { channel: "硅基流动 (SiliconFlow)", keywords: &["silicon"] },
    ChannelRule { channel: "火山方舟 (Volcengine)", keywords: &["volc", "ark", "ep-"] },
    ChannelRule { channel: "OpenRouter", keywords: &["openrouter"] },
    ChannelRule { channel: "阿里云百炼 (DashScope)", keywords: &["dashscope", "bailian", "aliyun"] },
    ChannelRule { channel: "Together AI", keywords: &["together"] },
    ChannelRule { channel: "Groq (LPU)", keywords: &["groq"] },
    ChannelRule { channel: "AWS Bedrock", keywords: &["bedrock"] },
    ChannelRule { channel: "GCP Vertex AI", keywords: &["vertex"] },
    ChannelRule { channel: "Azure AI Foundry", keywords: &["azure"] },
    ChannelRule { channel: "GPUStack 企业算力", keywords: &["gpustack", "10.232."] },
    ChannelRule { channel: "私有集群 (Self-Hosted/vLLM)", keywords: &["vllm", "sglang", "local", "ollama"] },
    ChannelRule { channel: "模拟环境 (Mock Sandbox)", keywords: &["mock"] },
];

struct CanonicalDisplayRule {
    display_name: &'static str,
    keywords: &'static [&'static str],
    exclusions: &'static [&'static str],
}

static CANONICAL_DISPLAY_RULES: &[CanonicalDisplayRule] = &[
    // 2026 Flagship Frontier
    CanonicalDisplayRule { display_name: "Claude-Opus-5.5", keywords: &["claude-opus-5.5", "claude-opus-5-5", "claude-5.5", "claude-5", "opus-5.5", "opus-5"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "GPT-6-Astra", keywords: &["gpt-6-astra", "gpt-6-sol", "gpt-6-luna", "gpt-6", "gpt6"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Kimi-K3", keywords: &["kimi-k3", "kimi_k3", "kimik3", "kimi-3", "kimi3"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Qwen-3.8-Max", keywords: &["qwen-3.8-max", "qwen-3.8", "qwen3.8", "qwen-3", "qwen3"], exclusions: &["qwq"] },
    CanonicalDisplayRule { display_name: "Gemini-3.8-Flash", keywords: &["gemini-3.8", "gemini-3.1", "gemini-3", "gemini3"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Llama-4-Maverick", keywords: &["llama-4-maverick", "llama-4-scout", "llama-4", "llama4"], exclusions: &[] },
    // Earlier Families
    CanonicalDisplayRule { display_name: "DeepSeek-V4", keywords: &["deepseek-v4", "deepseek_v4", "deepseekv4"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "DeepSeek-R1", keywords: &["deepseek-r1", "deepseek_r1", "deepseek-reasoner", "r1-distill"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "DeepSeek-V3", keywords: &["deepseek-v3", "deepseek-chat"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Grok-3 / Grok-4", keywords: &["grok-3", "grok3", "grok-4", "grok4", "grok"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Claude-3.7-Sonnet", keywords: &["claude-3-7", "claude-3.7", "claude-37"], exclusions: &["opus-5", "claude-5", "5.5"] },
    CanonicalDisplayRule { display_name: "Claude-3.5-Sonnet", keywords: &["claude-3-5", "claude-3.5", "claude-35"], exclusions: &["opus-5", "claude-5", "5.5"] },
    CanonicalDisplayRule { display_name: "OpenAI-o3-mini", keywords: &["o3-mini", "o3mini", "openai-o3", "o3"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "OpenAI-o1", keywords: &["o1-preview", "o1-mini", "openai-o1", "o1"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "GPT-4o-mini", keywords: &["gpt-4o-mini"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "GPT-4o", keywords: &["gpt-4o", "gpt4o", "gpt-5"], exclusions: &["gpt-6", "gpt6"] },
    CanonicalDisplayRule { display_name: "QwQ-32B", keywords: &["qwq-32b", "qwq_32b", "qwq"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Qwen-2.5-72B", keywords: &["qwen2.5-72b", "qwen-2.5-72b", "qwen-2.5"], exclusions: &["qwq", "qwen-3", "qwen3"] },
    CanonicalDisplayRule { display_name: "GLM-5 / GLM-4", keywords: &["glm-5", "glm5", "glm-4", "glm4", "glm"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Llama-3.3-70B", keywords: &["llama-3.3", "llama-3-3", "llama3.3"], exclusions: &["llama-4", "llama4"] },
    CanonicalDisplayRule { display_name: "Gemini-2.0-Pro", keywords: &["gemini-2.0-pro", "gemini-2-pro"], exclusions: &["gemini-3", "gemini3"] },
    CanonicalDisplayRule { display_name: "Gemini-2.0-Flash", keywords: &["gemini-2.0", "gemini-2"], exclusions: &["pro", "gemini-3", "gemini3"] },
    CanonicalDisplayRule { display_name: "Gemini-1.5-Pro", keywords: &["gemini-1.5"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Mock-Pro-v1", keywords: &["mock-pro"], exclusions: &[] },
    CanonicalDisplayRule { display_name: "Mock-Fast-v1", keywords: &["mock-fast"], exclusions: &[] },
];

struct TierRule {
    tier: &'static str,
    id_keywords: &'static [&'static str],
    categories: &'static [&'static str],
}

static TIER_RULES: &[TierRule] = &[
    TierRule {
        tier: "L5",
        id_keywords: &["_hard_", "putnam", "swe_hard", "jailbreak"],
        categories: &[],
    },
    TierRule {
        tier: "L4",
        id_keywords: &["agent_", "sec_", "devops_", "swe_", "react_", "tool_", "error_"],
        categories: &["swe", "agent", "devops", "security", "data_analyst"],
    },
    TierRule {
        tier: "L3",
        id_keywords: &["med_", "law_", "legal_", "fin_", "math_"],
        categories: &["medical", "legal", "finance", "science", "humanities", "math_logic"],
    },
    TierRule {
        tier: "L2",
        id_keywords: &["hum_", "sci_", "multi_", "needle_", "code_", "hallucination_"],
        categories: &["code_generation", "multilingual", "instruction", "structured_output", "long_context"],
    },
];

pub fn resolve_tier(test_case_id: &str, category: &str) -> &'static str {
    let lower_id = test_case_id.to_lowercase();
    TIER_RULES
        .iter()
        .find(|rule| {
            rule.id_keywords.iter().any(|&k| lower_id.contains(k))
                || rule.categories.contains(&category)
        })
        .map(|r| r.tier)
        .unwrap_or("L1")
}

pub fn resolve_canonical_and_channel(
    model_id: &str,
    model_name: &str,
    provider: Option<&str>,
) -> (String, String) {
    let lower_id = model_id.to_lowercase();
    let lower_name = model_name.to_lowercase();
    let lower_prov = provider.unwrap_or("").to_lowercase();
    let combined_str = format!("{} {} {}", lower_id, lower_name, lower_prov);

    // 1. Resolve Channel via declarative rules
    let channel = CHANNEL_RULES
        .iter()
        .find(|r| r.keywords.iter().any(|&k| combined_str.contains(k)))
        .map(|r| r.channel.to_string())
        .unwrap_or_else(|| "官方直连 (Official)".to_string());

    // 2. Resolve Canonical Model via declarative rules
    let canonical = CANONICAL_DISPLAY_RULES
        .iter()
        .find(|r| {
            let matched = r.keywords.iter().any(|&k| lower_id.contains(k) || lower_name.contains(k));
            let excluded = r.exclusions.iter().any(|&k| lower_id.contains(k) || lower_name.contains(k));
            matched && !excluded
        })
        .map(|r| r.display_name.to_string())
        .unwrap_or_else(|| model_name.to_string());

    (canonical, channel)
}

impl ModelBenchmarkSummary {
    pub fn compute(
        model_id: String,
        model_name: String,
        case_results: Vec<CaseResult>,
    ) -> Self {
        let (canonical_name, channel) = resolve_canonical_and_channel(&model_id, &model_name, None);
        let total_cases = case_results.len();
        if total_cases == 0 {
            return Self {
                model_id,
                model_name,
                provider: None,
                channel: Some(channel),
                canonical_name: Some(canonical_name),
                total_cases: 0,
                passed_cases: 0,
                overall_accuracy: 0.0,
                overall_score: 0.0,
                macro_accuracy: 0.0,
                macro_score: 0.0,
                weighted_composite_index: 0.0,
                elo_rating: 1200.0,
                efficiency_index: 0.0,
                tier_accuracy: HashMap::new(),
                l4_l5_frontier_accuracy: 0.0,
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

        // Compute Tier Breakdown (L1 ~ L5)
        let mut tier_map: HashMap<String, (usize, usize)> = HashMap::new();
        let mut l4_l5_passed = 0;
        let mut l4_l5_total = 0;

        for case in &case_results {
            let tier = case.difficulty.as_deref().unwrap_or_else(|| {
                resolve_tier(&case.test_case_id, case.category.as_str())
            });

            let entry = tier_map.entry(tier.to_string()).or_insert((0, 0));
            entry.0 += 1;
            if case.passed {
                entry.1 += 1;
            }

            if tier == "L4" || tier == "L5" {
                l4_l5_total += 1;
                if case.passed {
                    l4_l5_passed += 1;
                }
            }
        }

        let mut tier_accuracy = HashMap::new();
        for (tier, (total, passed)) in tier_map {
            if total > 0 {
                tier_accuracy.insert(tier, passed as f64 / total as f64);
            }
        }

        let l4_l5_frontier_accuracy = if l4_l5_total > 0 {
            l4_l5_passed as f64 / l4_l5_total as f64
        } else {
            overall_accuracy
        };

        Self {
            model_id,
            model_name,
            provider: None,
            channel: Some(channel),
            canonical_name: Some(canonical_name),
            total_cases,
            passed_cases,
            overall_accuracy,
            overall_score,
            macro_accuracy,
            macro_score,
            weighted_composite_index,
            elo_rating: 1200.0,
            efficiency_index,
            tier_accuracy,
            l4_l5_frontier_accuracy,
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
