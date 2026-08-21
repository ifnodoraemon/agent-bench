pub mod anthropic;
pub mod client;
pub mod gemini;
pub mod mock;
pub mod openai;
pub mod openai_responses;
pub mod types;

pub use anthropic::AnthropicClient;
pub use client::ModelClient;
pub use gemini::GeminiClient;
pub use mock::MockClient;
pub use openai::OpenAICompatibleClient;
pub use openai_responses::OpenAIResponsesClient;
pub use types::{
    extract_fallback_tool_calls, ApiProtocol, ChatMessage, FunctionCall, FunctionDefinition,
    ModelConfig, ModelResponse, ResponseFormat, Role, TokenUsage, ToolCall, ToolDefinition,
};

use anyhow::Result;
use std::sync::Arc;

pub fn create_client(config: ModelConfig) -> Result<Arc<dyn ModelClient>> {
    match config.protocol {
        ApiProtocol::OpenAiChat => Ok(Arc::new(OpenAICompatibleClient::new(config)?)),
        ApiProtocol::OpenAiResponse => Ok(Arc::new(OpenAIResponsesClient::new(config)?)),
        ApiProtocol::Anthropic => Ok(Arc::new(AnthropicClient::new(config)?)),
        ApiProtocol::Gemini => Ok(Arc::new(GeminiClient::new(config)?)),
        ApiProtocol::Mock => Ok(Arc::new(MockClient::new(config.id, config.model_name))),
    }
}
