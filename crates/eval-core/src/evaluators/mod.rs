pub mod code;
pub mod exact;
pub mod f1;
pub mod ifeval;
pub mod llm_judge;
pub mod multiple_choice;
pub mod numeric;
pub mod regex;
pub mod registry;
pub mod schema;
pub mod tool_call;

pub use code::CodeSandboxEvaluator;
pub use exact::ExactMatchEvaluator;
pub use f1::F1Evaluator;
pub use ifeval::IfEvalEvaluator;
pub use llm_judge::LlmJudgeEvaluator;
pub use multiple_choice::MultipleChoiceEvaluator;
pub use numeric::NumericEvaluator;
pub use regex::RegexEvaluator;
pub use registry::EvaluatorRegistry;
pub use schema::JsonSchemaEvaluator;
pub use tool_call::ToolCallEvaluator;

use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub passed: bool,
    pub score: f64, // 0.0 to 1.0
    pub reason: String,
    pub dimensions: Option<crate::metrics::DimensionScores>,
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub cost_usd: f64,
}

impl EvaluationResult {
    pub fn pass(reason: impl Into<String>) -> Self {
        Self {
            passed: true,
            score: 1.0,
            reason: reason.into(),
            dimensions: None,
            details: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            score: 0.0,
            reason: reason.into(),
            dimensions: None,
            details: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        }
    }

    pub fn partial(score: f64, reason: impl Into<String>) -> Self {
        let clamped = score.clamp(0.0, 1.0);
        Self {
            passed: clamped >= 0.5,
            score: clamped,
            reason: reason.into(),
            dimensions: None,
            details: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        }
    }

    pub fn with_dimensions(mut self, dimensions: crate::metrics::DimensionScores) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    pub fn with_judge_usage(
        mut self,
        prompt_tokens: u32,
        completion_tokens: u32,
        cost_usd: f64,
    ) -> Self {
        self.prompt_tokens = prompt_tokens;
        self.completion_tokens = completion_tokens;
        self.cost_usd = cost_usd;
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[async_trait]
pub trait Evaluator: Send + Sync {
    async fn evaluate(
        &self,
        test_case: &TestCase,
        response: &ModelResponse,
    ) -> Result<EvaluationResult>;
}
