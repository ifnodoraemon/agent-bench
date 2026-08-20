use super::types::{ChatMessage, ModelConfig, ModelResponse, ToolDefinition};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ModelClient: Send + Sync {
    /// Return the model configuration
    fn config(&self) -> &ModelConfig;

    /// Complete a chat request (streaming under the hood to capture TTFT and TPS)
    async fn chat_complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ModelResponse>;
}
