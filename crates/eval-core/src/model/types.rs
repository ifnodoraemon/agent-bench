use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiProtocol {
    #[serde(rename = "openai_chat", alias = "openai", alias = "chat_completions")]
    OpenAiChat,       // /v1/chat/completions (OpenAI, DeepSeek, Qwen, vLLM, Ollama, Groq, etc.)

    #[serde(rename = "openai_response", alias = "openai_responses", alias = "responses")]
    OpenAiResponse,   // /v1/responses (New OpenAI Responses API)

    #[serde(rename = "anthropic", alias = "claude", alias = "messages")]
    Anthropic,        // /v1/messages (Claude 3.5 Sonnet / Haiku)

    #[serde(rename = "gemini", alias = "google", alias = "generate_content")]
    Gemini,           // /v1beta/models/{model}:generateContent

    #[serde(rename = "mock")]
    Mock,             // Mock simulator for tests & offline validation
}

impl fmt::Display for ApiProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiProtocol::OpenAiChat => write!(f, "openai_chat"),
            ApiProtocol::OpenAiResponse => write!(f, "openai_response"),
            ApiProtocol::Anthropic => write!(f, "anthropic"),
            ApiProtocol::Gemini => write!(f, "gemini"),
            ApiProtocol::Mock => write!(f, "mock"),
        }
    }
}

impl FromStr for ApiProtocol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai_chat" | "openai" | "chat" | "v1/chat/completions" => Ok(ApiProtocol::OpenAiChat),
            "openai_response" | "openai_responses" | "responses" | "v1/responses" => Ok(ApiProtocol::OpenAiResponse),
            "anthropic" | "claude" | "messages" | "v1/messages" => Ok(ApiProtocol::Anthropic),
            "gemini" | "google" | "generate_content" => Ok(ApiProtocol::Gemini),
            "mock" => Ok(ApiProtocol::Mock),
            other => anyhow::bail!("Unsupported API protocol format: '{other}'. Choose from: 'openai_chat', 'openai_response', 'anthropic', 'gemini', 'mock'"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>, // DeepSeek-R1 / o1 thinking chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            reasoning_content: None,
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            reasoning_content: None,
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            reasoning_content: None,
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool_response(tool_call_id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            reasoning_content: None,
            name: Some(name.into()),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    pub fn function(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String, // "json_object" | "json_schema" | "text"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub protocol: ApiProtocol,     // OpenAiChat, OpenAiResponse, Anthropic, Gemini
    pub provider: String,          // 品牌/展示名 (如 "deepseek", "qwen", "openai")
    pub model_name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub custom_headers: Option<HashMap<String, String>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub response_format: Option<ResponseFormat>,
    pub reasoning_effort: Option<String>,      // "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max"
    pub extra_body: Option<HashMap<String, serde_json::Value>>, // Custom backend parameters
    pub price_per_input_million: Option<f64>,  // USD per 1M input tokens
    pub price_per_output_million: Option<f64>, // USD per 1M output tokens
}

impl ModelConfig {
    pub fn new(id: impl Into<String>, provider: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            protocol: ApiProtocol::OpenAiChat,
            provider: provider.into(),
            model_name: model_name.into(),
            base_url: None,
            api_key: None,
            custom_headers: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            response_format: None,
            reasoning_effort: None,
            extra_body: None,
            price_per_input_million: None,
            price_per_output_million: None,
        }
    }

    pub fn with_protocol(mut self, protocol: ApiProtocol) -> Self {
        self.protocol = protocol;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,    // Extracted thinking process (R1/o1)
    pub tool_calls: Option<Vec<ToolCall>>,
    pub usage: TokenUsage,
    pub total_duration: Duration,
    pub ttft: Option<Duration>,               // Time To First Token
    pub tokens_per_second: f64,
    pub estimated_cost_usd: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,       // "stop" | "length" | "tool_calls"
    pub raw_response: Option<serde_json::Value>,
}

/// Fallback parser that extracts structured tool calls if an open-source model (DeepSeek, GLM, Qwen, etc.)
/// generates markup/text tool calls instead of native JSON tool_calls in OpenAI API response.
pub fn extract_fallback_tool_calls(text: &str) -> Option<Vec<ToolCall>> {
    super::extractor::ToolCallExtractor::extract(text)
}
