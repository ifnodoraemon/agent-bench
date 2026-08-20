use super::client::ModelClient;
use super::types::{
    ChatMessage, FunctionCall, ModelConfig, ModelResponse, Role, TokenUsage, ToolCall,
    ToolDefinition,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::json;
use std::time::{Duration, Instant};

pub struct AnthropicClient {
    config: ModelConfig,
    http_client: reqwest::Client,
    endpoint: String,
}

impl AnthropicClient {
    pub fn new(config: ModelConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static("2023-06-01"),
        );
        if let Some(ref key) = config.api_key {
            if !key.is_empty() {
                headers.insert(
                    "x-api-key",
                    HeaderValue::from_str(key).context("Invalid Anthropic API key")?,
                );
            }
        }

        if let Some(ref custom_map) = config.custom_headers {
            for (k, v) in custom_map {
                if let (Ok(hn), Ok(hv)) = (reqwest::header::HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(v)) {
                    headers.insert(hn, hv);
                }
            }
        }

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(120))
            .build()?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
        let endpoint = if base_url.ends_with("/messages") {
            base_url
        } else {
            format!("{}/messages", base_url.trim_end_matches('/'))
        };

        Ok(Self {
            config,
            http_client,
            endpoint,
        })
    }

    fn calculate_cost(&self, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        let input_cost = self.config.price_per_input_million.unwrap_or(0.0)
            * (prompt_tokens as f64)
            / 1_000_000.0;
        let output_cost = self.config.price_per_output_million.unwrap_or(0.0)
            * (completion_tokens as f64)
            / 1_000_000.0;
        input_cost + output_cost
    }

    async fn send_request_with_retry(&self, body: &serde_json::Value) -> Result<reqwest::Response> {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = Duration::from_millis(500);

        loop {
            attempts += 1;
            let resp_result = self
                .http_client
                .post(&self.endpoint)
                .json(body)
                .send()
                .await;

            match resp_result {
                Ok(resp) => {
                    let status = resp.status();
                    if (status.as_u16() == 429 || status.is_server_error()) && attempts < max_attempts {
                        tokio::time::sleep(delay).await;
                        delay *= 2;
                        continue;
                    }
                    if !status.is_success() {
                        let err_text = resp.text().await.unwrap_or_default();
                        return Err(anyhow!("Anthropic API error ({status}): {err_text}"));
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    if attempts < max_attempts {
                        tokio::time::sleep(delay).await;
                        delay *= 2;
                        continue;
                    }
                    return Err(anyhow!("Anthropic network request failed after {attempts} attempts: {e}"));
                }
            }
        }
    }
}

#[async_trait]
impl ModelClient for AnthropicClient {
    fn config(&self) -> &ModelConfig {
        &self.config
    }

    async fn chat_complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ModelResponse> {
        let mut system_prompt = String::new();
        let mut anthropic_messages = Vec::new();

        for m in messages {
            match m.role {
                Role::System => {
                    if !system_prompt.is_empty() {
                        system_prompt.push('\n');
                    }
                    system_prompt.push_str(&m.content);
                }
                Role::User => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": m.content
                    }));
                }
                Role::Assistant => {
                    anthropic_messages.push(json!({
                        "role": "assistant",
                        "content": m.content
                    }));
                }
                Role::Tool => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": m.tool_call_id.clone().unwrap_or_default(),
                            "content": m.content
                        }]
                    }));
                }
            }
        }

        let mut body = json!({
            "model": self.config.model_name,
            "messages": anthropic_messages,
            "max_tokens": self.config.max_tokens.unwrap_or(4096),
            "stream": true
        });

        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        if let Some(temp) = self.config.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(tool_list) = tools {
            if !tool_list.is_empty() {
                let anthropic_tools: Vec<serde_json::Value> = tool_list
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.function.name,
                            "description": t.function.description,
                            "input_schema": t.function.parameters
                        })
                    })
                    .collect();
                body["tools"] = json!(anthropic_tools);
            }
        }

        let start_time = Instant::now();
        let resp = self.send_request_with_retry(&body).await?;

        let mut stream = resp.bytes_stream().eventsource();
        let mut first_token_time: Option<Duration> = None;
        let mut full_text = String::new();
        let mut tool_calls = Vec::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_input = String::new();

        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;

        while let Some(event_res) = stream.next().await {
            let event = event_res.context("Error in Anthropic SSE stream")?;
            let parsed: serde_json::Value = match serde_json::from_str(&event.data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let event_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match event_type {
                "message_start" => {
                    if let Some(msg) = parsed.get("message") {
                        if let Some(usage) = msg.get("usage") {
                            prompt_tokens = usage.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32;
                        }
                    }
                }
                "content_block_start" => {
                    if first_token_time.is_none() {
                        first_token_time = Some(start_time.elapsed());
                    }
                    if let Some(block) = parsed.get("content_block") {
                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            current_tool_id = block.get("id").and_then(|i| i.as_str()).unwrap_or_default().to_string();
                            current_tool_name = block.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_string();
                            current_tool_input.clear();
                        }
                    }
                }
                "content_block_delta" => {
                    if first_token_time.is_none() {
                        first_token_time = Some(start_time.elapsed());
                    }
                    if let Some(delta) = parsed.get("delta") {
                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            full_text.push_str(text);
                        } else if let Some(partial_json) = delta.get("partial_json").and_then(|pj| pj.as_str()) {
                            current_tool_input.push_str(partial_json);
                        }
                    }
                }
                "content_block_stop" => {
                    if !current_tool_name.is_empty() {
                        tool_calls.push(ToolCall {
                            id: current_tool_id.clone(),
                            tool_type: "function".to_string(),
                            function: FunctionCall {
                                name: current_tool_name.clone(),
                                arguments: current_tool_input.clone(),
                            },
                        });
                        current_tool_id.clear();
                        current_tool_name.clear();
                        current_tool_input.clear();
                    }
                }
                "message_delta" => {
                    if let Some(usage) = parsed.get("usage") {
                        completion_tokens = usage.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32;
                    }
                }
                _ => {}
            }
        }

        let total_duration = start_time.elapsed();
        let total_secs = total_duration.as_secs_f64();
        let tps = if total_secs > 0.0 && completion_tokens > 0 {
            completion_tokens as f64 / total_secs
        } else {
            0.0
        };

        let estimated_cost = self.calculate_cost(prompt_tokens, completion_tokens);

        Ok(ModelResponse {
            text: full_text,
            reasoning_content: None,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            total_duration,
            ttft: first_token_time,
            tokens_per_second: tps,
            estimated_cost_usd: estimated_cost,
            raw_response: None,
        })
    }
}
