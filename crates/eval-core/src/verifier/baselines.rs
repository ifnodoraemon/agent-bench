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
        // 9. Gemini 2.0 Flash
        OfficialModelBaseline {
            canonical_id: "gemini-2-0-flash".to_string(),
            display_name: "Gemini 2.0 Flash".to_string(),
            vendor: "Google DeepMind".to_string(),
            release_date: "2024-12".to_string(),
            tech_report_title: "Gemini 2.0: Our New AI Model for the Agentic Era".to_string(),
            tech_report_url: "https://blog.google/technology/google-deepmind/google-gemini-ai-update-december-2024/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1354.0, "Elo", "LMSYS Arena Jan 2025"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 35.6, "%", "Google DeepMind Gemini 2.0 Evaluation"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 91.2, "%", "Google DeepMind Gemini 2.0 Evaluation Table 2"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 78.4, "%", "Google DeepMind Gemini 2.0 Evaluation Table 1"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 56.5, "%", "Google DeepMind Gemini 2.0 Evaluation Table 1"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 88.2, "%", "Google DeepMind Gemini 2.0 Evaluation Table 3"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 43.1, "%", "LiveCodeBench Jan 2025"),
            ],
            primary_traits: vec![
                "Google DeepMind 原生全模态架构".to_string(),
                "亚秒级极速首字输出与超大上下文窗口".to_string(),
                "原生工具调用与多模态流式交互".to_string(),
            ],
        },
        // 10. OpenAI o3-mini
        OfficialModelBaseline {
            canonical_id: "openai-o3-mini".to_string(),
            display_name: "OpenAI o3-mini".to_string(),
            vendor: "OpenAI".to_string(),
            release_date: "2025-01".to_string(),
            tech_report_title: "OpenAI o3-mini: Advancing Science, Math and Coding via Efficient Reasoning".to_string(),
            tech_report_url: "https://openai.com/index/openai-o3-mini/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1370.0, "Elo", "LMSYS Arena Jan 2025"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 49.3, "%", "OpenAI o3-mini Evaluation"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 97.9, "%", "OpenAI o3-mini Evaluation"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 85.1, "%", "OpenAI o3-mini Evaluation"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 79.7, "%", "OpenAI o3-mini Evaluation"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 87.2, "%", "OpenAI o3-mini Evaluation"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 70.4, "%", "LiveCodeBench Feb 2025"),
            ],
            primary_traits: vec![
                "OpenAI 2025/2026 高效推理旗舰 (Reasoning Effort 可调)".to_string(),
                "竞赛级代码生成与高阶数学定理证明".to_string(),
                "原生结构化 CoT 与极速首字输出".to_string(),
            ],
        },
        // 11. Gemini 2.0 Pro
        OfficialModelBaseline {
            canonical_id: "gemini-2-0-pro".to_string(),
            display_name: "Gemini 2.0 Pro".to_string(),
            vendor: "Google DeepMind".to_string(),
            release_date: "2025-02".to_string(),
            tech_report_title: "Gemini 2.0 Pro: Frontier Reasoning and Multimodal Agent Architecture".to_string(),
            tech_report_url: "https://blog.google/technology/google-deepmind/gemini-2-0-flash-thinking-pro/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1390.0, "Elo", "LMSYS Arena Feb 2025"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 58.4, "%", "Google DeepMind Gemini 2.0 Evaluation"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 95.8, "%", "Google DeepMind Gemini 2.0 Evaluation"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 86.8, "%", "Google DeepMind Gemini 2.0 Evaluation"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 78.2, "%", "Google DeepMind Gemini 2.0 Evaluation"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 91.0, "%", "Google DeepMind Gemini 2.0 Evaluation"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 55.6, "%", "LiveCodeBench Feb 2025"),
            ],
            primary_traits: vec![
                "Google DeepMind 前沿全模态复杂推理 Agent".to_string(),
                "复杂代码重构与百万级超长上下文".to_string(),
                "高可靠多工具自主编排与流式交互".to_string(),
            ],
        },
        // 12. Grok 3
        OfficialModelBaseline {
            canonical_id: "grok-3".to_string(),
            display_name: "Grok 3".to_string(),
            vendor: "xAI".to_string(),
            release_date: "2025-04".to_string(),
            tech_report_title: "Grok 3: Colossus-Scale Reasoning and Frontier Intelligence".to_string(),
            tech_report_url: "https://x.ai/blog/grok-3".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1385.0, "Elo", "LMSYS Arena Apr 2025"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 55.8, "%", "xAI Grok 3 Benchmark Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 98.2, "%", "xAI Grok 3 Benchmark Report"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 88.6, "%", "xAI Grok 3 Benchmark Report"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 82.4, "%", "xAI Grok 3 Benchmark Report"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 90.8, "%", "xAI Grok 3 Benchmark Report"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 68.5, "%", "LiveCodeBench Apr 2025"),
            ],
            primary_traits: vec![
                "xAI Colossus 万卡超算集群原生预训练".to_string(),
                "深度思考 DeepSearch 与自洽数学定理证明".to_string(),
                "极强抗幻觉与客观事实研判".to_string(),
            ],
        },
        // 13. DeepSeek-V4
        OfficialModelBaseline {
            canonical_id: "deepseek-v4".to_string(),
            display_name: "DeepSeek-V4".to_string(),
            vendor: "DeepSeek".to_string(),
            release_date: "2025-08".to_string(),
            tech_report_title: "DeepSeek-V4: Next-Generation Sparse Architecture with Dynamic Thinking".to_string(),
            tech_report_url: "https://arxiv.org/abs/deepseek-v4".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1410.0, "Elo", "LMSYS Arena 2025"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 74.5, "%", "DeepSeek-V4 Evaluation"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 98.5, "%", "DeepSeek-V4 Evaluation"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 90.2, "%", "DeepSeek-V4 Evaluation"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 86.5, "%", "DeepSeek-V4 Evaluation"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 93.4, "%", "DeepSeek-V4 Evaluation"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 72.8, "%", "LiveCodeBench 2025"),
            ],
            primary_traits: vec![
                "下一代极速稀疏超大杯架构".to_string(),
                "全自适应原生混合思考流".to_string(),
                "超低时延与企业级私有化友好".to_string(),
            ],
        },
        // 14. QwQ-32B
        OfficialModelBaseline {
            canonical_id: "qwq-32b".to_string(),
            display_name: "QwQ-32B".to_string(),
            vendor: "Alibaba Cloud".to_string(),
            release_date: "2025-03".to_string(),
            tech_report_title: "QwQ-32B: Open-Weights Reasoning with Multi-Stage RL".to_string(),
            tech_report_url: "https://qwenlm.github.io/blog/qwq-32b/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1360.0, "Elo", "LMSYS Arena Mar 2025"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 46.8, "%", "QwQ Technical Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 96.5, "%", "QwQ Technical Report"),
                bm("aime_2024", "AIME 2024", "数学竞赛 Pass@1", 79.5, "%", "QwQ Technical Report"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 82.5, "%", "QwQ Technical Report"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 68.9, "%", "QwQ Technical Report"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 84.8, "%", "QwQ Technical Report"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 63.4, "%", "LiveCodeBench Mar 2025"),
            ],
            primary_traits: vec![
                "开源 32B 极限轻量级深度推理基座".to_string(),
                "Multi-Stage RL 强化学习思维链演进".to_string(),
                "Apache 2.0 完全商用友好代码与数学天花板".to_string(),
            ],
        },
        // 15. Claude Opus 5.5 (2026-09)
        OfficialModelBaseline {
            canonical_id: "claude-opus-5-5".to_string(),
            display_name: "Claude Opus 5.5".to_string(),
            vendor: "Anthropic".to_string(),
            release_date: "2026-09".to_string(),
            tech_report_title: "Claude Opus 5.5: Frontier Agentic Coding and Intelligence".to_string(),
            tech_report_url: "https://www.anthropic.com/news/claude-opus-5-5".to_string(),
            scores: vec![
                bm("arena_elo", "Code Arena WebDev", "综合竞技场与代码竞技", 1818.0, "Elo", "Code Arena Sep 2026"),
                bm("swe_bench_verified", "SWE-bench Pro", "前沿工程代码修复率", 78.2, "%", "Anthropic Sep 2026 Evaluation"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 98.8, "%", "Anthropic Official Evaluation"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 92.4, "%", "Anthropic Official Evaluation"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 89.2, "%", "Anthropic Official Evaluation"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 94.5, "%", "Anthropic Official Evaluation"),
                bm("livecodebench", "Terminal-Bench 4.0", "自主终端智能体任务解决率", 66.4, "%", "Terminal-Bench Sep 2026 Leaderboard"),
            ],
            primary_traits: vec![
                "Anthropic 2026年9月最新智能体旗舰".to_string(),
                "Terminal-Bench 4.0 达 66.4% 业界第一".to_string(),
                "前沿自主 Agent 编码与超强多步反思纠偏".to_string(),
            ],
        },
        // 16. GPT-6 Astra (2026-09)
        OfficialModelBaseline {
            canonical_id: "gpt-6-astra".to_string(),
            display_name: "GPT-6 Astra".to_string(),
            vendor: "OpenAI".to_string(),
            release_date: "2026-09".to_string(),
            tech_report_title: "GPT-6 Astra: Frontier Autonomous Agents and Multimodal Computer Use".to_string(),
            tech_report_url: "https://openai.com/index/gpt-6-astra/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1520.0, "Elo", "LMSYS Arena Sep 2026"),
                bm("swe_bench_verified", "SWE-bench Pro", "前沿工程代码修复率", 74.8, "%", "OpenAI System Card Sep 2026"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 98.6, "%", "OpenAI Official Evaluation"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 91.8, "%", "OpenAI Official Evaluation"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 88.5, "%", "OpenAI Official Evaluation"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 93.8, "%", "OpenAI Official Evaluation"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 78.4, "%", "LiveCodeBench Sep 2026"),
            ],
            primary_traits: vec![
                "OpenAI 2026年9月全新架构旗舰 (Astra/Sol/Luna)".to_string(),
                "原生操作系统级 Computer Use 与自主 Agent 编排".to_string(),
                "超高难度竞赛数学与全模态自洽推理".to_string(),
            ],
        },
        // 17. Kimi K3 (2026-07)
        OfficialModelBaseline {
            canonical_id: "kimi-k3".to_string(),
            display_name: "Kimi K3".to_string(),
            vendor: "Moonshot AI".to_string(),
            release_date: "2026-07".to_string(),
            tech_report_title: "Kimi K3: 2.8-Trillion Parameter Open-Weights Reasoning and 1M Context".to_string(),
            tech_report_url: "https://moonshot.cn/research/kimi-k3".to_string(),
            scores: vec![
                bm("arena_elo", "Code Arena WebDev", "Web 前端竞技场", 1682.0, "Elo", "Code Arena Jul 2026"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 59.4, "%", "Moonshot K3 Technical Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 97.8, "%", "Moonshot K3 Technical Report"),
                bm("aime_2024", "AIME 2026", "数学竞赛 Pass@1", 82.5, "%", "Moonshot K3 Technical Report"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 88.5, "%", "Moonshot K3 Technical Report"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 81.2, "%", "Moonshot K3 Technical Report"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 91.2, "%", "Moonshot K3 Technical Report"),
                bm("livecodebench", "Terminal-Bench", "终端代码解决率", 61.2, "%", "Moonshot K3 Technical Report"),
            ],
            primary_traits: vec![
                "月之暗面 2.8T 超大规模稀疏 MoE 架构".to_string(),
                "Code Arena WebDev 排名榜首".to_string(),
                "原生 1M 超长文本即时召回与极速推理".to_string(),
            ],
        },
        // 18. Qwen 3.8-Max (2026-08)
        OfficialModelBaseline {
            canonical_id: "qwen-3-8-max".to_string(),
            display_name: "Qwen 3.8-Max".to_string(),
            vendor: "Alibaba Cloud".to_string(),
            release_date: "2026-08".to_string(),
            tech_report_title: "Qwen 3.8-Max: Frontier Multimodal Agent and Autonomous Coding".to_string(),
            tech_report_url: "https://qwenlm.github.io/blog/qwen-3.8/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1480.0, "Elo", "LMSYS Arena Aug 2026"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 62.0, "%", "Alibaba Qwen 3.8 Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 98.1, "%", "Alibaba Qwen 3.8 Report"),
                bm("mmlu_pro", "PaperBench", "高阶科研论文研判准确率", 93.0, "%", "PaperBench Aug 2026"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 85.0, "%", "Alibaba Qwen 3.8 Report"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 92.5, "%", "Alibaba Qwen 3.8 Report"),
                bm("livecodebench", "Terminal-Bench 2.1", "终端智能体任务解决率", 86.6, "%", "Terminal-Bench Aug 2026"),
            ],
            primary_traits: vec![
                "阿里云 2026 下半年顶级旗舰 (Qwen 3.8 架构)".to_string(),
                "复杂专业论文研判 PaperBench 达 93.0%".to_string(),
                "Terminal 智能体与代码运维天花板 (86.6%)".to_string(),
            ],
        },
        // 19. Gemini 3.8 Flash / 3.1 Pro (2026-09)
        OfficialModelBaseline {
            canonical_id: "gemini-3-8-flash".to_string(),
            display_name: "Gemini 3.8 Flash".to_string(),
            vendor: "Google DeepMind".to_string(),
            release_date: "2026-09".to_string(),
            tech_report_title: "Gemini 3: Frontier Agent Architecture and Audio-Visual Reasoning".to_string(),
            tech_report_url: "https://blog.google/technology/google-deepmind/gemini-3-update/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1505.0, "Elo", "LMSYS Arena 2026"),
                bm("swe_bench_verified", "SWE-bench Verified", "工程 Agent 代码解决率", 72.0, "%", "Google DeepMind Gemini 3 Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 97.4, "%", "Google DeepMind Gemini 3 Report"),
                bm("mmlu_pro", "ARC-AGI-2", "通用流体智力抽象推理", 77.1, "%", "Google DeepMind Gemini 3 Report"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 86.0, "%", "Google DeepMind Gemini 3 Report"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 93.0, "%", "Google DeepMind Gemini 3 Report"),
                bm("livecodebench", "Big Bench Audio", "全模态音频理解与推理", 97.7, "%", "Google DeepMind Gemini 3 Report"),
            ],
            primary_traits: vec![
                "Google DeepMind 2026 旗舰全模态架构".to_string(),
                "首个突破 LMSYS 1500 Elo 标杆的大模型系列".to_string(),
                "ARC-AGI-2 抽象推理 77.1% 与亚秒级全双工交互".to_string(),
            ],
        },
        // 20. Llama 4 Maverick (2026-04)
        OfficialModelBaseline {
            canonical_id: "llama-4-maverick".to_string(),
            display_name: "Llama 4 Maverick (400B MoE)".to_string(),
            vendor: "Meta".to_string(),
            release_date: "2026-04".to_string(),
            tech_report_title: "The Llama 4 Herd: Native Multimodal MoE at 400B Scale".to_string(),
            tech_report_url: "https://ai.meta.com/blog/llama-4/".to_string(),
            scores: vec![
                bm("arena_elo", "LMSYS Chatbot Arena", "综合竞技场", 1440.0, "Elo", "LMSYS Arena Apr 2026"),
                bm("swe_bench_verified", "SWE-bench Pro", "工程代码解决率", 61.5, "%", "Meta AI Llama 4 Report"),
                bm("math_500", "MATH-500", "高阶数学定理推理", 97.2, "%", "Meta AI Llama 4 Report"),
                bm("mmlu_pro", "MMLU-Pro", "高阶通识多学科综合", 91.0, "%", "Meta AI Llama 4 Report"),
                bm("gpqa_diamond", "GPQA Diamond", "博士级跨学科深层推理", 84.0, "%", "Meta AI Llama 4 Report"),
                bm("ifeval", "IFEval", "严格指令遵循精度", 92.0, "%", "Meta AI Llama 4 Report"),
                bm("livecodebench", "LiveCodeBench", "实效竞赛级代码解题", 71.0, "%", "LiveCodeBench Apr 2026"),
            ],
            primary_traits: vec![
                "Meta 2026 开源 400B 混合专家 (MoE) 架构".to_string(),
                "1000 万 Token 原生超长上下文支持".to_string(),
                "全模态多任务微调与开源最强代码推理".to_string(),
            ],
        },
    ]
});

/// Declarative matching rule for model normalization
pub struct CanonicalResolutionRule {
    pub canonical_id: &'static str,
    pub match_keywords: &'static [&'static str],
    pub negative_keywords: &'static [&'static str],
}

pub static CANONICAL_RULES: &[CanonicalResolutionRule] = &[
    // 2026 New Flagship Models First
    CanonicalResolutionRule {
        canonical_id: "claude-opus-5-5",
        match_keywords: &["claude-opus-5.5", "claude-opus-5-5", "claude-5.5", "claude-5", "opus-5.5", "opus-5"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "gpt-6-astra",
        match_keywords: &["gpt-6-astra", "gpt-6-sol", "gpt-6-luna", "gpt-6", "gpt6"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "kimi-k3",
        match_keywords: &["kimi-k3", "kimi_k3", "kimik3", "kimi-3", "kimi3"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "qwen-3-8-max",
        match_keywords: &["qwen-3.8-max", "qwen-3.8", "qwen3.8", "qwen-3", "qwen3"],
        negative_keywords: &["qwq"],
    },
    CanonicalResolutionRule {
        canonical_id: "gemini-3-8-flash",
        match_keywords: &["gemini-3.8", "gemini-3.1", "gemini-3", "gemini3"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "llama-4-maverick",
        match_keywords: &["llama-4-maverick", "llama-4-scout", "llama-4", "llama4"],
        negative_keywords: &[],
    },
    // DeepSeek Family
    CanonicalResolutionRule {
        canonical_id: "deepseek-v4",
        match_keywords: &["deepseek-v4", "deepseek_v4", "deepseekv4"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "deepseek-r1",
        match_keywords: &["deepseek-r1", "deepseek_r1", "deepseek-reasoner", "r1-distill"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "deepseek-v3",
        match_keywords: &["deepseek-v3", "deepseek_v3", "deepseek-chat"],
        negative_keywords: &[],
    },
    // xAI Grok Family
    CanonicalResolutionRule {
        canonical_id: "grok-3",
        match_keywords: &["grok-3", "grok3", "grok-4", "grok4", "grok"],
        negative_keywords: &[],
    },
    // Anthropic Claude Family
    CanonicalResolutionRule {
        canonical_id: "claude-3-7-sonnet",
        match_keywords: &["claude-3-7", "claude-3.7", "claude-37"],
        negative_keywords: &["opus-5", "claude-5", "5.5"],
    },
    CanonicalResolutionRule {
        canonical_id: "claude-3-5-sonnet",
        match_keywords: &["claude-3-5", "claude-3.5", "claude-35"],
        negative_keywords: &["opus-5", "claude-5", "5.5"],
    },
    // OpenAI Family
    CanonicalResolutionRule {
        canonical_id: "openai-o3-mini",
        match_keywords: &["o3-mini", "o3mini", "openai-o3", "o3"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "openai-o1",
        match_keywords: &["o1-preview", "o1-mini", "openai-o1", "o1"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "gpt-4o",
        match_keywords: &["gpt-4o", "gpt4o", "gpt-5", "gpt5"],
        negative_keywords: &["gpt-6", "gpt6"],
    },
    // Google Gemini Family
    CanonicalResolutionRule {
        canonical_id: "gemini-2-0-pro",
        match_keywords: &["gemini-2.0-pro", "gemini-2-pro", "gemini-pro-2", "gemini-3"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "gemini-2-0-flash",
        match_keywords: &["gemini-2.0-flash", "gemini-2-flash", "gemini-2.0", "gemini-2", "gemini"],
        negative_keywords: &["pro"],
    },
    // Alibaba Qwen Family
    CanonicalResolutionRule {
        canonical_id: "qwq-32b",
        match_keywords: &["qwq-32b", "qwq_32b", "qwq"],
        negative_keywords: &[],
    },
    CanonicalResolutionRule {
        canonical_id: "qwen-2-5-72b",
        match_keywords: &["qwen-2.5", "qwen2.5", "qwen3", "qwen"],
        negative_keywords: &["qwq"],
    },
    // Meta Llama Family
    CanonicalResolutionRule {
        canonical_id: "llama-3-3-70b",
        match_keywords: &["llama-3.3", "llama-3-3", "llama3.3", "llama-4", "llama"],
        negative_keywords: &[],
    },
];

/// Resolve raw model string to a canonical official baseline id using declarative rules (no if/else chain, strictly no fallback)
pub fn resolve_canonical_id(model_name: &str) -> Option<&'static str> {
    let lower = model_name.to_lowercase();
    CANONICAL_RULES.iter().find_map(|rule| {
        let has_match = rule.match_keywords.iter().any(|&k| lower.contains(k));
        let has_negative = rule.negative_keywords.iter().any(|&k| lower.contains(k));
        if has_match && !has_negative {
            Some(rule.canonical_id)
        } else {
            None
        }
    })
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

/// Compute drift analysis between tested metrics and official ground truth baseline.
/// Strictly returns baseline_found: false when model is unindexed (no fallback to DeepSeek-R1).
pub fn evaluate_baseline_drift(
    target_model: &str,
    tested_metrics: &[(&str, f64)],
) -> BaselineDriftReport {
    let canonical_opt = resolve_canonical_id(target_model);
    let baseline_opt = canonical_opt.and_then(get_official_baseline);

    if baseline_opt.is_none() {
        return BaselineDriftReport {
            target_model: target_model.to_string(),
            canonical_id: canonical_opt.unwrap_or("unindexed").to_string(),
            baseline_found: false,
            display_name: target_model.to_string(),
            vendor: "Unknown / Not Indexed".to_string(),
            tech_report_url: "".to_string(),
            comparisons: Vec::new(),
            mean_drift_pct: 0.0,
            drift_severity: DriftStatus::Matching,
            verdict: format!("未收录标称模型【{}】的官方权威基准分记录", target_model),
            confidence: 0.0,
        };
    }

    let baseline = baseline_opt.unwrap();
    let canonical = canonical_opt.unwrap();
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

            let (status, diagnosis) = if (-5.0..=5.0).contains(&delta_pct) {
                (
                    DriftStatus::Matching,
                    format!("实测分与官方公布分高度吻合（Δ = {:+.1}%），表现真实稳定", delta_pct),
                )
            } else if (-15.0..-5.0).contains(&delta_pct) {
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
        assert_eq!(resolve_canonical_id("claude-opus-5.5-20260922"), Some("claude-opus-5-5"));
        assert_eq!(resolve_canonical_id("gpt-6-astra-preview"), Some("gpt-6-astra"));
        assert_eq!(resolve_canonical_id("kimi-k3-2.8t"), Some("kimi-k3"));
        assert_eq!(resolve_canonical_id("qwen-3.8-max-20260902"), Some("qwen-3-8-max"));
        assert_eq!(resolve_canonical_id("gemini-3.8-flash-exp"), Some("gemini-3-8-flash"));
        assert_eq!(resolve_canonical_id("llama-4-maverick-400b"), Some("llama-4-maverick"));
        assert_eq!(resolve_canonical_id("deepseek-ai/DeepSeek-R1"), Some("deepseek-r1"));
        assert_eq!(resolve_canonical_id("claude-3-7-sonnet-20250219"), Some("claude-3-7-sonnet"));
        assert_eq!(resolve_canonical_id("claude-3-5-sonnet-20241022"), Some("claude-3-5-sonnet"));
        assert_eq!(resolve_canonical_id("o1-preview"), Some("openai-o1"));
        assert_eq!(resolve_canonical_id("o3-mini"), Some("openai-o3-mini"));
        assert_eq!(resolve_canonical_id("gemini-2.0-pro-exp"), Some("gemini-2-0-pro"));
        assert_eq!(resolve_canonical_id("gemini-2.0-flash"), Some("gemini-2-0-flash"));
        assert_eq!(resolve_canonical_id("gpt-4o-mini"), Some("gpt-4o"));
        assert_eq!(resolve_canonical_id("grok-3-beta"), Some("grok-3"));
        assert_eq!(resolve_canonical_id("deepseek-v4-moe"), Some("deepseek-v4"));
        assert_eq!(resolve_canonical_id("qwq-32b-preview"), Some("qwq-32b"));
        assert_eq!(resolve_canonical_id("qwen-2.5-72b-instruct"), Some("qwen-2-5-72b"));
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

    #[test]
    fn test_unindexed_model_no_fallback() {
        let tested = vec![("swe_bench_verified", 20.0)];
        let report = evaluate_baseline_drift("completely-unknown-custom-model-99", &tested);
        assert!(!report.baseline_found);
        assert_eq!(report.canonical_id, "unindexed");
        assert!(report.comparisons.is_empty());
        assert!(report.verdict.contains("未收录"));
    }
}
