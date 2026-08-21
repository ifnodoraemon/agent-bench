use super::client::ModelClient;
use super::types::{ChatMessage, ModelConfig, ModelResponse, TokenUsage, ToolCall, ToolDefinition};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct MockClient {
    config: ModelConfig,
    responses: Arc<Mutex<HashMap<String, (String, Option<Vec<ToolCall>>)>>>,
    default_response: String,
    simulated_delay: Duration,
}

impl MockClient {
    pub fn new(id: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            config: ModelConfig::new(id, "mock", model_name),
            responses: Arc::new(Mutex::new(HashMap::new())),
            default_response: "Mock default answer".to_string(),
            simulated_delay: Duration::from_millis(50),
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.simulated_delay = delay;
        self
    }

    pub fn with_default_response(mut self, response: impl Into<String>) -> Self {
        self.default_response = response.into();
        self
    }

    pub fn add_canned_response(
        &self,
        prompt_keyword: impl Into<String>,
        response: impl Into<String>,
        tool_calls: Option<Vec<ToolCall>>,
    ) {
        let mut map = self.responses.lock().unwrap();
        map.insert(prompt_keyword.into(), (response.into(), tool_calls));
    }
}

#[async_trait]
impl ModelClient for MockClient {
    fn config(&self) -> &ModelConfig {
        &self.config
    }

    async fn chat_complete(
        &self,
        messages: &[ChatMessage],
        _tools: Option<&[ToolDefinition]>,
    ) -> Result<ModelResponse> {
        if self.simulated_delay > Duration::ZERO {
            tokio::time::sleep(self.simulated_delay).await;
        }

        let last_prompt = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        let map = self.responses.lock().unwrap();

        let (text, tool_calls) = map
            .iter()
            .find(|(k, _)| last_prompt.contains(k.as_str()))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| (self.default_response.clone(), None));

        let prompt_tokens = (last_prompt.len() / 4).max(5) as u32;
        let completion_tokens = (text.len() / 4).max(5) as u32;

        Ok(ModelResponse {
            text,
            reasoning_content: None,
            tool_calls,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            total_duration: self.simulated_delay,
            ttft: Some(self.simulated_delay / 2),
            tokens_per_second: if self.simulated_delay.as_secs_f64() > 0.0 {
                completion_tokens as f64 / self.simulated_delay.as_secs_f64()
            } else {
                100.0
            },
            estimated_cost_usd: 0.0,
            finish_reason: Some("stop".to_string()),
            raw_response: None,
        })
    }
}
