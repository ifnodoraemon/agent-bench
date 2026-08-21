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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub category: Category,
    #[serde(default)]
    pub tags: Vec<String>,
    pub prompt: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub reference_answer: Option<String>,
    pub eval_type: EvaluationType,
    #[serde(default)]
    pub criteria: Option<String>,
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    #[serde(default)]
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
        let diff_mult: f64 = if let Some(diff) = self.metadata.get("difficulty").and_then(|v| v.as_str()) {
            match diff.to_lowercase().as_str() {
                "easy" | "simple" => 0.8,
                "medium" => 1.0,
                "hard" | "complex_agent" | "expert" => 1.5,
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
}
