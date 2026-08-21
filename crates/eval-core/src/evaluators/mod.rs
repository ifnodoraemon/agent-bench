pub mod code;
pub mod exact;
pub mod llm_judge;
pub mod regex;
pub mod registry;
pub mod schema;

pub use code::CodeSandboxEvaluator;
pub use exact::ExactMatchEvaluator;
pub use llm_judge::LlmJudgeEvaluator;
pub use regex::RegexEvaluator;
pub use registry::EvaluatorRegistry;
pub use schema::JsonSchemaEvaluator;

use crate::dataset::TestCase;
use crate::model::ModelResponse;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub passed: bool,
    pub score: f64,                     // 0.0 to 1.0
    pub reason: String,
    pub dimensions: Option<crate::metrics::DimensionScores>,
    pub details: Option<serde_json::Value>,
}

impl EvaluationResult {
    pub fn pass(reason: impl Into<String>) -> Self {
        Self {
            passed: true,
            score: 1.0,
            reason: reason.into(),
            dimensions: None,
            details: None,
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            score: 0.0,
            reason: reason.into(),
            dimensions: None,
            details: None,
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
        }
    }

    pub fn with_dimensions(mut self, dimensions: crate::metrics::DimensionScores) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[async_trait]
pub trait Evaluator: Send + Sync {
    async fn evaluate(&self, test_case: &TestCase, response: &ModelResponse) -> Result<EvaluationResult>;
}
