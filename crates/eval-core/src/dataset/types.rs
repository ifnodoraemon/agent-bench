use crate::model::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Foundation,
    Agent,
    Safety,
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
            Category::Performance => "performance",
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

        // 2. Difficulty preset if specified
        if let Some(diff) = self.metadata.get("difficulty").and_then(|v| v.as_str()) {
            match diff.to_lowercase().as_str() {
                "easy" | "simple" => return 60,
                "medium" => return 90,
                "hard" => return 150,
                "complex_agent" | "expert" => return 300,
                _ => {}
            }
        }

        // 3. Adaptive calculation by evaluation type and turn count (giving ample room for deep CoT reasoning)
        match self.eval_type {
            EvaluationType::ExactMatch | EvaluationType::Regex => 120,
            EvaluationType::JsonSchema | EvaluationType::CodeExecution => 120,
            EvaluationType::LlmJudge => 150,
            EvaluationType::AgentTrajectory => {
                let turns = self.max_turns.unwrap_or(5);
                (60 + (turns as u64) * 35).clamp(120, 360)
            }
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
}
