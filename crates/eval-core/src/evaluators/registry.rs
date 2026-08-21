use super::{
    CodeSandboxEvaluator, EvaluationResult, Evaluator, ExactMatchEvaluator, JsonSchemaEvaluator,
    RegexEvaluator,
};
use crate::dataset::{EvaluationType, TestCase};
use crate::model::ModelResponse;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Dynamic Evaluator Registry managing evaluation strategies according to the Open-Closed Principle.
#[derive(Default)]
pub struct EvaluatorRegistry {
    strategies: HashMap<EvaluationType, Arc<dyn Evaluator>>,
}

impl EvaluatorRegistry {
    /// Initialize registry with default built-in evaluation strategies
    pub fn new() -> Self {
        let mut registry = Self {
            strategies: HashMap::new(),
        };
        registry.register(EvaluationType::ExactMatch, Arc::new(ExactMatchEvaluator::default()));
        registry.register(EvaluationType::Regex, Arc::new(RegexEvaluator));
        registry.register(EvaluationType::JsonSchema, Arc::new(JsonSchemaEvaluator));
        registry.register(EvaluationType::CodeExecution, Arc::new(CodeSandboxEvaluator::default()));
        registry
    }

    /// Register or override an evaluation strategy for a specific EvaluationType
    pub fn register(&mut self, eval_type: EvaluationType, evaluator: Arc<dyn Evaluator>) {
        self.strategies.insert(eval_type, evaluator);
    }

    /// Retrieve an evaluation strategy
    pub fn get(&self, eval_type: &EvaluationType) -> Option<Arc<dyn Evaluator>> {
        self.strategies.get(eval_type).cloned()
    }

    /// Direct evaluation dispatch via registered strategy
    pub async fn evaluate(&self, test_case: &TestCase, response: &ModelResponse) -> Result<EvaluationResult> {
        if let Some(strategy) = self.get(&test_case.eval_type) {
            strategy.evaluate(test_case, response).await
        } else {
            Err(anyhow!(
                "No registered evaluation strategy found for eval_type: {:?}",
                test_case.eval_type
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_registry_exact_match() {
        let registry = EvaluatorRegistry::new();
        let tc = TestCase {
            id: "test_01".to_string(),
            name: None,
            category: Category::Foundation,
            tags: vec![],
            prompt: "What is 2+2?".to_string(),
            system_prompt: None,
            reference_answer: Some("4".to_string()),
            eval_type: EvaluationType::ExactMatch,
            criteria: None,
            schema: None,
            test_code: None,
            tools: None,
            max_turns: None,
            metadata: Default::default(),
        };

        let resp = ModelResponse {
            text: "4".to_string(),
            reasoning_content: None,
            tool_calls: None,
            usage: Default::default(),
            ttft: None,
            total_duration: Duration::from_millis(10),
            tokens_per_second: 100.0,
            estimated_cost_usd: 0.0,
            finish_reason: None,
            raw_response: None,
        };

        let res = registry.evaluate(&tc, &resp).await.expect("Evaluation should succeed");
        assert!(res.passed);
        assert_eq!(res.score, 1.0);
    }
}
