use crate::model::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Foundation,
    Agent,
    Safety,
    Swe,
    Devops,
    Security,
    DataAnalyst,
    MathLogic,
    Multilingual,
    LongContext,
    Instruction,
    StructuredOutput,
    Medical,
    Legal,
    Finance,
    Science,
    Humanities,
    Performance,
    #[serde(untagged)]
    Custom(String),
}

impl Category {
    pub fn as_str(&self) -> &str {
        match self {
            Category::Foundation => "foundation",
            Category::Agent => "agent",
            Category::Safety => "safety",
            Category::Swe => "swe",
            Category::Devops => "devops",
            Category::Security => "security",
            Category::DataAnalyst => "data_analyst",
            Category::MathLogic => "math_logic",
            Category::Multilingual => "multilingual",
            Category::LongContext => "long_context",
            Category::Instruction => "instruction",
            Category::StructuredOutput => "structured_output",
            Category::Medical => "medical",
            Category::Legal => "legal",
            Category::Finance => "finance",
            Category::Science => "science",
            Category::Humanities => "humanities",
            Category::Performance => "performance",
            Category::Custom(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Category::Foundation => "Foundation (基础基准)",
            Category::Agent => "Agent Core (智能体核心)",
            Category::Safety => "Safety & Defense (安全防御)",
            Category::Swe => "Coding & SWE (软件工程)",
            Category::Devops => "DevOps & Cloud (云原生运维)",
            Category::Security => "Security & Audit (网络安全攻防)",
            Category::DataAnalyst => "Data & SQL (数据分析与BI)",
            Category::MathLogic => "Math & Logic (数理逻辑推理)",
            Category::Multilingual => "Multilingual (多语言与NLP)",
            Category::LongContext => "Long Context (长文本与大海捞针)",
            Category::Instruction => "Instruction (指令遵循与约束)",
            Category::StructuredOutput => "Structured Output (结构化输出)",
            Category::Medical => "Medicine & Healthcare (医疗健康)",
            Category::Legal => "Law & Jurisprudence (法律法规)",
            Category::Finance => "Finance & Economics (金融商科)",
            Category::Science => "Natural Sciences (自然科学)",
            Category::Humanities => "Humanities & Philosophy (人文社科)",
            Category::Performance => "Performance (吞吐与延迟)",
            Category::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationType {
    ExactMatch,
    Regex,
    JsonSchema,
    CodeExecution,
    LlmJudge,
    AgentTrajectory,
}

impl EvaluationType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "exact_match" | "exact" => Some(EvaluationType::ExactMatch),
            "regex" => Some(EvaluationType::Regex),
            "json_schema" | "schema" | "json" => Some(EvaluationType::JsonSchema),
            "code_execution" | "code" | "sandbox" => Some(EvaluationType::CodeExecution),
            "llm_judge" | "judge" => Some(EvaluationType::LlmJudge),
            "agent_trajectory" | "agent" | "trajectory" | "tools" => Some(EvaluationType::AgentTrajectory),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub category: Category,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub prompt: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default, alias = "expected_output", alias = "expected", alias = "answer")]
    pub reference_answer: Option<String>,
    pub eval_type: EvaluationType,
    #[serde(default, alias = "rubric", alias = "judge_criteria")]
    pub criteria: Option<String>,
    #[serde(default, alias = "json_schema")]
    pub schema: Option<serde_json::Value>,
    #[serde(default, alias = "code", alias = "script")]
    pub test_code: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(default)]
    pub max_turns: Option<usize>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TestCase {
    /// Calculate adaptive timeout in seconds based on difficulty, eval_type, and max_turns
    pub fn timeout_seconds(&self) -> u64 {
        // 1. Explicit metadata timeout
        if let Some(t) = self
            .metadata
            .get("timeout_secs")
            .or_else(|| self.metadata.get("timeout"))
            .and_then(|v| v.as_u64())
        {
            return t.clamp(5, 600);
        }

        // 2. Base timeout by evaluation type
        let base_secs: u64 = match self.eval_type {
            EvaluationType::ExactMatch | EvaluationType::Regex => 150,
            EvaluationType::JsonSchema | EvaluationType::CodeExecution => 180,
            EvaluationType::LlmJudge => 180,
            EvaluationType::AgentTrajectory => {
                let turns = self.max_turns.unwrap_or(6) as u64;
                (90 + turns * 45).clamp(180, 480)
            }
        };

        // 3. Difficulty multiplier
        let diff_mult: f64 = if let Some(diff) = self
            .difficulty
            .as_deref()
            .or_else(|| self.metadata.get("difficulty").and_then(|v| v.as_str()))
        {
            match diff.to_lowercase().as_str() {
                "l1" | "easy" | "simple" => 0.8,
                "l2" | "medium" => 1.0,
                "l3" | "hard" => 1.3,
                "l4" | "complex_agent" | "expert" => 1.6,
                "l5" | "olympiad" | "frontier" => 2.0,
                _ => 1.0,
            }
        } else if self.tags.iter().any(|t| {
            matches!(
                t.as_str(),
                "olympiad" | "deep_reasoning" | "complex_refactor" | "logic" | "puzzle" | "math"
            )
        }) {
            1.5
        } else {
            1.0
        };

        let calculated = ((base_secs as f64) * diff_mult).round() as u64;
        calculated.clamp(60, 600)
    }

    /// Resolve canonical difficulty tier (L1 to L5)
    pub fn resolved_tier(&self) -> &str {
        if let Some(ref d) = self.difficulty {
            return d.as_str();
        }
        if let Some(d) = self.metadata.get("difficulty").and_then(|v| v.as_str()) {
            return d;
        }
        if self.id.contains("_hard_") || self.id.contains("putnam") || self.id.contains("swe_hard") {
            "L5"
        } else if self.id.contains("agent_")
            || self.id.contains("sec_")
            || self.id.contains("devops_")
            || self.tags.iter().any(|t| t == "hard" || t == "complex_agent")
        {
            "L4"
        } else if matches!(
            self.category,
            Category::Medical
                | Category::Legal
                | Category::Finance
                | Category::Science
                | Category::Humanities
                | Category::MathLogic
        ) {
            "L3"
        } else if self.tags.iter().any(|t| t == "simple" || t == "easy" || t == "sanity") {
            "L1"
        } else {
            "L2"
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub test_cases: Vec<TestCase>,
}

impl Dataset {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            version: Some("1.0.0".to_string()),
            test_cases: Vec::new(),
        }
    }

    pub fn filter_by_category(&self, cat: &Category) -> Self {
        Self {
            name: format!("{}-{}", self.name, cat.as_str()),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| &c.category == cat)
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_tag(&self, tag: &str) -> Self {
        Self {
            name: format!("{}-tag-{}", self.name, tag),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| c.tags.iter().any(|t| t == tag))
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_difficulty(&self, diff_pattern: &str) -> Self {
        let targets: Vec<String> = diff_pattern
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            name: format!("{}-diff-{}", self.name, diff_pattern),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| {
                    let d = c.resolved_tier().to_lowercase();
                    targets.iter().any(|t| {
                        if t == "frontier" {
                            d == "l4" || d == "l5" || d == "expert" || d == "olympiad"
                        } else {
                            d == *t || c.tags.iter().any(|tag| tag.to_lowercase() == *t)
                        }
                    })
                })
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_eval_type(&self, eval_type: &EvaluationType) -> Self {
        Self {
            name: format!("{}-{:?}", self.name, eval_type),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| &c.eval_type == eval_type)
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_ids(&self, ids: &std::collections::HashSet<String>) -> Self {
        Self {
            name: format!("{}-filtered-ids", self.name),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| ids.contains(&c.id))
                .cloned()
                .collect(),
        }
    }

    pub fn limit(&self, max_cases: usize) -> Self {
        Self {
            name: format!("{}-limit-{}", self.name, max_cases),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self.test_cases.iter().take(max_cases).cloned().collect(),
        }
    }

    pub fn sample(&self, ratio: f64, seed: Option<u64>) -> Self {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        let ratio = ratio.clamp(0.0, 1.0);
        let sample_count = ((self.test_cases.len() as f64) * ratio).ceil() as usize;

        let mut cases = self.test_cases.clone();
        if let Some(s) = seed {
            let mut rng = rand::rngs::StdRng::seed_from_u64(s);
            cases.shuffle(&mut rng);
        } else {
            let mut rng = rand::thread_rng();
            cases.shuffle(&mut rng);
        }
        cases.truncate(sample_count);

        Self {
            name: format!("{}-sample-{:.2}", self.name, ratio),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: cases,
        }
    }
}
