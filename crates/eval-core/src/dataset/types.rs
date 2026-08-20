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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
