use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkMetric {
    pub benchmark_id: String,
    pub benchmark_name: String,
    pub category: String,
    pub score: f64,
    pub unit: String,
    pub citation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficialModelBaseline {
    pub canonical_id: String,
    pub display_name: String,
    pub vendor: String,
    pub release_date: String,
    pub tech_report_title: String,
    pub tech_report_url: String,
    pub scores: Vec<BenchmarkMetric>,
    pub primary_traits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriftStatus {
    Matching,          // Within ±5.0%
    MinorDeficit,      // -5.0% ~ -15.0% (Typical quantization drop INT8/FP8)
    SevereDeficit,     // < -15.0% (Severe downgrade / counterfeit)
    SuspiciousOverfit, // > +10.0% (Possible benchmark leak or overfit prompt)
}

impl DriftStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Matching => "matching",
            Self::MinorDeficit => "minor_deficit",
            Self::SevereDeficit => "severe_deficit",
            Self::SuspiciousOverfit => "suspicious_overfit",
        }
    }

    pub fn display_badge(&self) -> &'static str {
        match self {
            Self::Matching => "✓ 吻合基线 (±5%)",
            Self::MinorDeficit => "⚠️ 轻微偏离 (-5%~-15%)",
            Self::SevereDeficit => "🚨 严重衰减 (<-15%)",
            Self::SuspiciousOverfit => "❓ 异常高分 (+10%)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftMetricComparison {
    pub benchmark_id: String,
    pub benchmark_name: String,
    pub category: String,
    pub official_score: f64,
    pub tested_score: f64,
    pub unit: String,
    pub delta: f64,
    pub delta_pct: f64,
    pub status: DriftStatus,
    pub diagnosis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineDriftReport {
    pub target_model: String,
    pub canonical_id: String,
    pub baseline_found: bool,
    pub display_name: String,
    pub vendor: String,
    pub tech_report_url: String,
    pub comparisons: Vec<DriftMetricComparison>,
    pub mean_drift_pct: f64,
    pub drift_severity: DriftStatus,
    pub verdict: String,
    pub confidence: f64,
}

fn bm(id: &str, name: &str, cat: &str, score: f64, unit: &str, cit: &str) -> BenchmarkMetric {
    BenchmarkMetric {
        benchmark_id: id.to_string(),
        benchmark_name: name.to_string(),
        category: cat.to_string(),
        score,
        unit: unit.to_string(),
        citation: cit.to_string(),
    }
}

pub static OFFICIAL_BASELINES: LazyLock<Vec<OfficialModelBaseline>> = LazyLock::new(|| {
    vec![
        // 1. DeepSeek-R1
        OfficialModelBaseline {
            canonical_id: "deepseek-r1".to_string(),
            display_name: "DeepSeek-R1".to_string(),
            vendor: "DeepSeek".to_string(),
            release_date: "2025-01".to_string(),
            tech_report_title: "DeepSeek-R1: Incentivizing Reasoning Capability in LLMs via RL".to_string(),
            tech_report_url: "https://arxiv.org/abs/2501.12948".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1364.0, "Elo", "LMSYS Chatbot Arena Leaderboard (2025-01)"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 49.2, "%", "DeepSeek-R1 Technical Report Table 2"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 97.3, "%", "DeepSeek-R1 Technical Report Table 1"),
                bm("aime_2024", "AIME 2024", "数学竞赛 Pass@1", 79.8, "%", "DeepSeek-R1 Technical Report Table 1"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 84.0, "%", "DeepSeek-R1 Technical Report Table 3"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 71.5, "%", "DeepSeek-R1 Technical Report Table 1"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 83.3, "%", "DeepSeek-R1 Technical Report Table 4"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 65.9, "%", "LiveCodeBench Jan 2025 Benchmark"),
            ],
            primary_traits: vec![
                "强制 <think> 深度思考链签名".to_string(),
                "具备极强直觉抑制与复杂代数反思纠偏".to_string(),
                "FP8 原生无损精度尾数完整".to_string(),
                "拒绝盲目阿谀 (低谄媚倾向)".to_string(),
            ],
        },
        // 2. DeepSeek-V3
        OfficialModelBaseline {
            canonical_id: "deepseek-v3".to_string(),
            display_name: "DeepSeek-V3".to_string(),
            vendor: "DeepSeek".to_string(),
            release_date: "2024-12".to_string(),
            tech_report_title: "DeepSeek-V3 Technical Report".to_string(),
            tech_report_url: "https://arxiv.org/abs/2412.19437".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1317.0, "Elo", "LMSYS Chatbot Arena Leaderboard"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 42.0, "%", "DeepSeek-V3 Tech Report Table 4"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 90.2, "%", "DeepSeek-V3 Tech Report Table 2"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 75.9, "%", "DeepSeek-V3 Tech Report Table 2"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 59.1, "%", "DeepSeek-V3 Tech Report Table 2"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 82.7, "%", "DeepSeek-V3 Tech Report Table 3"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 40.5, "%", "LiveCodeBench Dec 2024"),
            ],
            primary_traits: vec![
                "MoE 671B 稀疏架构".to_string(),
                "超高速首字生成与高性价比".to_string(),
                "Multi-head Latent Attention (MLA)".to_string(),
            ],
        },
        // 3. Claude 3.7 Sonnet
        OfficialModelBaseline {
            canonical_id: "claude-3-7-sonnet".to_string(),
            display_name: "Claude 3.7 Sonnet".to_string(),
            vendor: "Anthropic".to_string(),
            release_date: "2025-02".to_string(),
            tech_report_title: "Claude 3.7 Sonnet and Claude Code Announcement".to_string(),
            tech_report_url: "https://www.anthropic.com/news/claude-3-7-sonnet".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1380.0, "Elo", "LMSYS Arena Feb 2025"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 70.3, "%", "Anthropic Claude 3.7 Technical Evaluation"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 96.2, "%", "Anthropic Official Evaluation"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 87.2, "%", "Anthropic Official Evaluation"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 84.8, "%", "Anthropic Official Evaluation"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 91.5, "%", "Anthropic Official Evaluation"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 59.2, "%", "LiveCodeBench Feb 2025"),
            ],
            primary_traits: vec![
                "混动推理 (Hybrid Reasoning: Standard + Extended Thinking)".to_string(),
                "业界标杆代码重构与大型工程 Agent 能力".to_string(),
                "极严谨的多约束格式把控".to_string(),
            ],
        },
        // 4. Claude 3.5 Sonnet
        OfficialModelBaseline {
            canonical_id: "claude-3-5-sonnet".to_string(),
            display_name: "Claude 3.5 Sonnet (20241022)".to_string(),
            vendor: "Anthropic".to_string(),
            release_date: "2024-10".to_string(),
            tech_report_title: "Claude 3.5 Model Card Addendum".to_string(),
            tech_report_url: "https://www.anthropic.com/claude-3-5".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1335.0, "Elo", "LMSYS Arena 2024"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 49.0, "%", "Anthropic Oct 2024 Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 78.3, "%", "Anthropic Oct 2024 Report"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 78.0, "%", "Anthropic Oct 2024 Report"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 65.0, "%", "Anthropic Oct 2024 Report"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 88.0, "%", "Anthropic Oct 2024 Report"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 41.2, "%", "LiveCodeBench Nov 2024"),
            ],
            primary_traits: vec![
                "Anthropic 创作者自洽声明".to_string(),
                "代码工具调用极其稳定自然".to_string(),
                "优秀的上下文抓取与长文本理解".to_string(),
            ],
        },
        // 5. OpenAI o1
        OfficialModelBaseline {
            canonical_id: "openai-o1".to_string(),
            display_name: "OpenAI o1".to_string(),
            vendor: "OpenAI".to_string(),
            release_date: "2024-12".to_string(),
            tech_report_title: "Learning to Reason with LLMs (OpenAI o1 System Card)".to_string(),
            tech_report_url: "https://openai.com/index/learning-to-reason-with-llms/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1358.0, "Elo", "LMSYS Arena 2024"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 48.9, "%", "OpenAI o1 System Card Dec 2024"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 96.4, "%", "OpenAI o1 System Card Table 1"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 83.3, "%", "OpenAI o1 System Card"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 75.7, "%", "OpenAI o1 System Card"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 85.5, "%", "OpenAI o1 System Card"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 62.1, "%", "LiveCodeBench Dec 2024"),
            ],
            primary_traits: vec![
                "隐式链式思考推理 (Internal CoT)".to_string(),
                "高密度逻辑推演与自我质疑回溯".to_string(),
                "极强直觉偏误抑制力".to_string(),
            ],
        },
        // 6. GPT-4o
        OfficialModelBaseline {
            canonical_id: "gpt-4o".to_string(),
            display_name: "GPT-4o (2024-11-20)".to_string(),
            vendor: "OpenAI".to_string(),
            release_date: "2024-11".to_string(),
            tech_report_title: "Hello GPT-4o Technical Specification".to_string(),
            tech_report_url: "https://openai.com/index/hello-gpt-4o/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1310.0, "Elo", "LMSYS Arena 2024"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 38.8, "%", "OpenAI System Card Update"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 74.6, "%", "OpenAI Official Benchmark"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 72.6, "%", "OpenAI Official Benchmark"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 53.6, "%", "OpenAI Official Benchmark"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 84.3, "%", "OpenAI Official Benchmark"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 33.8, "%", "LiveCodeBench 2024"),
            ],
            primary_traits: vec![
                "全模态原生 Omni 架构".to_string(),
                "快速通用推理与多轮对话自洽".to_string(),
                "严格系统级引导遵循".to_string(),
            ],
        },
        // 7. Qwen-2.5-72B-Instruct
        OfficialModelBaseline {
            canonical_id: "qwen-2-5-72b".to_string(),
            display_name: "Qwen 2.5 72B Instruct".to_string(),
            vendor: "Alibaba Cloud".to_string(),
            release_date: "2024-09".to_string(),
            tech_report_title: "Qwen2.5: A Party of Foundation Models".to_string(),
            tech_report_url: "https://arxiv.org/abs/2412.15115".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1290.0, "Elo", "LMSYS Arena"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 31.2, "%", "Qwen2.5 Technical Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 83.1, "%", "Qwen2.5 Technical Report Table 2"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 69.4, "%", "Qwen2.5 Technical Report Table 2"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 49.0, "%", "Qwen2.5 Technical Report Table 2"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 84.1, "%", "Qwen2.5 Technical Report Table 3"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 31.0, "%", "LiveCodeBench Oct 2024"),
            ],
            primary_traits: vec![
                "中英双语顶尖通识认知".to_string(),
                "代码与数学开源基座天花板".to_string(),
                "结构化 JSON 生成稳定".to_string(),
            ],
        },
        // 8. Llama-3.3-70B-Instruct
        OfficialModelBaseline {
            canonical_id: "llama-3-3-70b".to_string(),
            display_name: "Llama 3.3 70B Instruct".to_string(),
            vendor: "Meta".to_string(),
            release_date: "2024-12".to_string(),
            tech_report_title: "The Llama 3 Herd of Models & Llama 3.3 Announcement".to_string(),
            tech_report_url: "https://ai.meta.com/blog/llama-3-3/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1275.0, "Elo", "LMSYS Arena Dec 2024"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 28.5, "%", "Meta AI Benchmark Card"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 73.8, "%", "Meta AI Benchmark Card"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 68.3, "%", "Meta AI Benchmark Card"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 50.5, "%", "Meta AI Benchmark Card"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 87.5, "%", "Meta AI Benchmark Card"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 27.6, "%", "LiveCodeBench Dec 2024"),
            ],
            primary_traits: vec![
                "Meta 开源旗舰指令模型".to_string(),
                "高质量全语种对齐与低偏见".to_string(),
                "优秀的通用工具与智能体支持".to_string(),
            ],
        },
    ]
});

/// Resolve raw model string to a canonical official baseline id
pub fn resolve_canonical_id(model_name: &str) -> Option<&'static str> {
    let lower = model_name.to_lowercase();

    // DeepSeek
    if lower.contains("deepseek") {
        if lower.contains("r1") || lower.contains("reasoner") {
            return Some("deepseek-r1");
        }
        if lower.contains("v3") || lower.contains("chat") {
            return Some("deepseek-v3");
        }
        return Some("deepseek-r1");
    }

    // Claude
    if lower.contains("claude") {
        if lower.contains("3-7") || lower.contains("3.7") {
            return Some("claude-3-7-sonnet");
        }
        if lower.contains("3-5") || lower.contains("3.5") || lower.contains("sonnet") {
            return Some("claude-3-5-sonnet");
        }
        return Some("claude-3-5-sonnet");
    }

    // OpenAI
    if lower.contains("o1") {
        return Some("openai-o1");
    }
    if lower.contains("gpt-4o") || lower.contains("gpt4o") {
        return Some("gpt-4o");
    }

    // Qwen
    if lower.contains("qwen") {
        if lower.contains("72b") || lower.contains("72") {
            return Some("qwen-2-5-72b");
        }
        return Some("qwen-2-5-72b");
    }

    // Llama
    if lower.contains("llama") {
        if lower.contains("70b") || lower.contains("70") {
            return Some("llama-3-3-70b");
        }
        return Some("llama-3-3-70b");
    }

    None
}

/// Retrieve an official baseline by its canonical ID
pub fn get_official_baseline(canonical_id: &str) -> Option<&'static OfficialModelBaseline> {
    OFFICIAL_BASELINES
        .iter()
        .find(|b| b.canonical_id == canonical_id)
}

/// Retrieve all official baselines
pub fn get_all_baselines() -> &'static [OfficialModelBaseline] {
    &OFFICIAL_BASELINES
}

/// Compute drift analysis between tested metrics and official ground truth baseline
pub fn evaluate_baseline_drift(
    target_model: &str,
    tested_metrics: &[(&str, f64)],
) -> BaselineDriftReport {
    let canonical = resolve_canonical_id(target_model).unwrap_or("deepseek-r1");
    let baseline_opt = get_official_baseline(canonical);

    if baseline_opt.is_none() {
        return BaselineDriftReport {
            target_model: target_model.to_string(),
            canonical_id: canonical.to_string(),
            baseline_found: false,
            display_name: target_model.to_string(),
            vendor: "Unknown".to_string(),
            tech_report_url: "".to_string(),
            comparisons: Vec::new(),
            mean_drift_pct: 0.0,
            drift_severity: DriftStatus::Matching,
            verdict: format!("未找到标称模型【{}】的权威官方基准分记录", target_model),
            confidence: 0.0,
        };
    }

    let baseline = baseline_opt.unwrap();
    let mut comparisons = Vec::new();
    let mut drift_sum = 0.0;
    let mut drift_count = 0usize;

    for (b_id, tested_val) in tested_metrics {
        if let Some(official_metric) = baseline.scores.iter().find(|m| m.benchmark_id == *b_id) {
            let official = official_metric.score;
            let delta = tested_val - official;
            let delta_pct = if official > 0.0 {
                (delta / official) * 100.0
            } else {
                0.0
            };

            let (status, diagnosis) = if delta_pct >= -5.0 && delta_pct <= 5.0 {
                (
                    DriftStatus::Matching,
                    format!("实测分与官方公布分高度吻合（Δ = {:+.1}%），表现真实稳定", delta_pct),
                )
            } else if delta_pct < -5.0 && delta_pct >= -15.0 {
                (
                    DriftStatus::MinorDeficit,
                    format!("实测分较官方微降 {:+.1}%，推测为节点量化（如 INT8/FP8）或不同并发温度所致", delta_pct),
                )
            } else if delta_pct < -15.0 {
                (
                    DriftStatus::SevereDeficit,
                    format!("严重警报：实测分大幅落后官方公布达 {:+.1}%，疑似极度压缩量化（INT4）或低阶套壳！", delta_pct),
                )
            } else {
                (
                    DriftStatus::SuspiciousOverfit,
                    format!("异常高分：实测分高出官方 {:+.1}%，需排查题目是否泄露过拟合或存在作弊", delta_pct),
                )
            };

            drift_sum += delta_pct;
            drift_count += 1;

            comparisons.push(DriftMetricComparison {
                benchmark_id: b_id.to_string(),
                benchmark_name: official_metric.benchmark_name.to_string(),
                category: official_metric.category.to_string(),
                official_score: official,
                tested_score: *tested_val,
                unit: official_metric.unit.to_string(),
                delta: (delta * 10.0).round() / 10.0,
                delta_pct: (delta_pct * 10.0).round() / 10.0,
                status,
                diagnosis,
            });
        }
    }

    let mean_drift = if drift_count > 0 {
        drift_sum / drift_count as f64
    } else {
        0.0
    };

    let (severity, verdict) = if comparisons.is_empty() {
        (
            DriftStatus::Matching,
            format!("已收录【{}】官方基线，但当前测试暂无对应权威指标比对", baseline.display_name),
        )
    } else if mean_drift < -15.0 {
        (
            DriftStatus::SevereDeficit,
            format!(
                "🚨 严重漂移警告：被测端点在权威基线上的综合实测表现较官方技术报告衰减达 {:.1}%，存在重大套壳中转或劣质降配嫌疑！",
                mean_drift.abs()
            ),
        )
    } else if mean_drift < -5.0 {
        (
            DriftStatus::MinorDeficit,
            format!(
                "⚠️ 轻微性能损耗：被测端点综合指标较官方公布漂移 {:.1}%，推测属于标准商业优化节点或中度量化托管方案。",
                mean_drift.abs()
            ),
        )
    } else if mean_drift > 10.0 {
        (
            DriftStatus::SuspiciousOverfit,
            format!(
                "❓ 异常高偏离：被测端点实测分超出官方标准 {:.1}%，请核验是否存在题目记忆或 Prompt 注入偏置。",
                mean_drift
            ),
        )
    } else {
        (
            DriftStatus::Matching,
            format!(
                "🛡️ 官方保真吻合：被测端点核心指标与官方公布数据偏离度仅 {:.1}%，高度契合原厂正品标准！",
                mean_drift
            ),
        )
    };

    let confidence = if drift_count >= 3 {
        95.0
    } else if drift_count >= 1 {
        80.0
    } else {
        50.0
    };

    BaselineDriftReport {
        target_model: target_model.to_string(),
        canonical_id: canonical.to_string(),
        baseline_found: true,
        display_name: baseline.display_name.to_string(),
        vendor: baseline.vendor.to_string(),
        tech_report_url: baseline.tech_report_url.to_string(),
        comparisons,
        mean_drift_pct: (mean_drift * 10.0).round() / 10.0,
        drift_severity: severity,
        verdict,
        confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_canonical_id() {
        assert_eq!(resolve_canonical_id("deepseek-ai/DeepSeek-R1"), Some("deepseek-r1"));
        assert_eq!(resolve_canonical_id("claude-3-7-sonnet-20250219"), Some("claude-3-7-sonnet"));
        assert_eq!(resolve_canonical_id("claude-3-5-sonnet-20241022"), Some("claude-3-5-sonnet"));
        assert_eq!(resolve_canonical_id("o1-preview"), Some("openai-o1"));
        assert_eq!(resolve_canonical_id("gpt-4o-mini"), Some("gpt-4o"));
    }

    #[test]
    fn test_evaluate_baseline_drift() {
        let tested = vec![
            ("swe_bench_verified", 48.5),
            ("math_500", 96.8),
            ("arena_elo", 1360.0),
        ];

        let report = evaluate_baseline_drift("DeepSeek-R1", &tested);
        assert!(report.baseline_found);
        assert_eq!(report.comparisons.len(), 3);
        assert_eq!(report.drift_severity, DriftStatus::Matching);
        assert!(report.mean_drift_pct.abs() < 5.0);
    }

    #[test]
    fn test_severe_drift_detection() {
        let tested = vec![
            ("swe_bench_verified", 20.0),
            ("math_500", 50.0),
        ];

        let report = evaluate_baseline_drift("DeepSeek-R1", &tested);
        assert_eq!(report.drift_severity, DriftStatus::SevereDeficit);
        assert!(report.mean_drift_pct < -15.0);
    }
}
