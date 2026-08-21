use super::client::ModelClient;
use super::types::{
    ChatMessage, FunctionCall, ModelConfig, ModelResponse, Role, TokenUsage, ToolCall,
    ToolDefinition,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use std::time::{Duration, Instant};

pub struct OpenAIResponsesClient {
    config: ModelConfig,
    http_client: reqwest::Client,
    endpoint: String,
}

impl OpenAIResponsesClient {
    pub fn new(config: ModelConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(ref key) = config.api_key {
            if !key.is_empty() && key != "none" {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", key))
                        .context("Invalid API key header")?,
                );
            }
        }

        if let Some(ref custom_map) = config.custom_headers {
            for (k, v) in custom_map {
                if let (Ok(hn), Ok(hv)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(v)) {
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
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let clean_base = base_url
            .trim_end_matches('/')
            .trim_end_matches("/chat/completions")
            .trim_end_matches("/responses");

        let endpoint = format!("{clean_base}/responses");

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
                        return Err(anyhow!("OpenAI Responses API error ({status}): {err_text}"));
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    if attempts < max_attempts {
                        tokio::time::sleep(delay).await;
                        delay *= 2;
                        continue;
                    }
                    return Err(anyhow!("Network request failed after {attempts} attempts: {e}"));
                }
            }
        }
    }
}

#[async_trait]
impl ModelClient for OpenAIResponsesClient {
    fn config(&self) -> &ModelConfig {
        &self.config
    }

    async fn chat_complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ModelResponse> {
        let mut instructions = String::new();
        let mut input_items = Vec::new();

        for m in messages {
            match m.role {
                Role::System => {
                    if !instructions.is_empty() {
                        instructions.push('\n');
                    }
                    instructions.push_str(&m.content);
                }
                Role::User => {
                    input_items.push(json!({
                        "role": "user",
                        "content": m.content
                    }));
                }
                Role::Assistant => {
                    input_items.push(json!({
                        "role": "assistant",
                        "content": m.content
                    }));
                }
                Role::Tool => {
                    input_items.push(json!({
                        "type": "function_call_output",
                        "call_id": m.tool_call_id.clone().unwrap_or_default(),
                        "output": m.content
                    }));
                }
            }
        }

        let mut body = json!({
            "model": self.config.model_name,
            "input": input_items,
            "stream": true
        });

        if !instructions.is_empty() {
            body["instructions"] = json!(instructions);
        }
        if let Some(temp) = self.config.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = self.config.max_tokens {
            body["max_output_tokens"] = json!(max_tokens);
        }
        if let Some(tool_list) = tools {
            if !tool_list.is_empty() {
                let resp_tools: Vec<serde_json::Value> = tool_list
                    .iter()
                    .map(|t| {
                        json!({
                            "type": "function",
                            "name": t.function.name,
                            "description": t.function.description,
                            "parameters": t.function.parameters
                        })
                    })
                    .collect();
                body["tools"] = json!(resp_tools);
            }
        }

        let start_time = Instant::now();
        let resp = self.send_request_with_retry(&body).await?;

        let mut stream = resp.bytes_stream().eventsource();
        let mut first_token_time: Option<Duration> = None;
        let mut full_text = String::new();
        let mut reasoning_text = String::new();
        let mut tool_calls = Vec::new();
        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;

        while let Some(event_res) = stream.next().await {
            let event = match event_res {
                Ok(ev) => ev,
                Err(err) => {
                    if !full_text.is_empty() || !reasoning_text.is_empty() || !tool_calls.is_empty() {
                        tracing::info!("Gracefully salvaging OpenAI Responses output despite SSE stream termination: {err}");
                        break;
                    } else {
                        return Err(anyhow::anyhow!("OpenAI Responses SSE stream interrupted before content received: {err}"));
                    }
                }
            };
            if event.data == "[DONE]" {
                break;
            }

            let parsed: serde_json::Value = match serde_json::from_str(&event.data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let event_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");

            // Parse text delta
            if event_type == "response.text.delta" || event_type == "response.output_text.delta" {
                if first_token_time.is_none() {
                    first_token_time = Some(start_time.elapsed());
                }
                if let Some(delta) = parsed.get("delta").and_then(|d| d.as_str()) {
                    full_text.push_str(delta);
                }
            } else if event_type == "response.reasoning.delta" {
                if first_token_time.is_none() {
                    first_token_time = Some(start_time.elapsed());
                }
                if let Some(delta) = parsed.get("delta").and_then(|d| d.as_str()) {
                    reasoning_text.push_str(delta);
                }
            } else if event_type == "response.output_item.added" || event_type == "response.output_item.done" {
                if let Some(item) = parsed.get("item") {
                    let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if item_type == "function_call" {
                        if first_token_time.is_none() {
                            first_token_time = Some(start_time.elapsed());
                        }
                        let call_id = item.get("call_id").and_then(|id| id.as_str()).unwrap_or("").to_string();
                        let fn_name = item.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                        let fn_args = item.get("arguments").map(|a| {
                            if a.is_string() { a.as_str().unwrap().to_string() } else { a.to_string() }
                        }).unwrap_or_else(|| "{}".to_string());

                        if !fn_name.is_empty() && !tool_calls.iter().any(|tc: &ToolCall| tc.id == call_id) {
                            tool_calls.push(ToolCall {
                                id: if call_id.is_empty() { format!("call_{}", tool_calls.len()) } else { call_id },
                                tool_type: "function".to_string(),
                                function: FunctionCall {
                                    name: fn_name,
                                    arguments: fn_args,
                                },
                            });
                        }
                    }
                }
            } else if event_type == "response.completed" || event_type == "response.done" {
                if let Some(resp_obj) = parsed.get("response") {
                    if let Some(usage) = resp_obj.get("usage") {
                        if let Some(pt) = usage.get("input_tokens").and_then(|t| t.as_u64()) {
                            prompt_tokens = pt as u32;
                        }
                        if let Some(ct) = usage.get("output_tokens").and_then(|t| t.as_u64()) {
                            completion_tokens = ct as u32;
                        }
                    }
                }
            }
        }

        let total_duration = start_time.elapsed();

        if completion_tokens == 0 && !full_text.is_empty() {
            let total_chars = full_text.len() + reasoning_text.len();
            completion_tokens = (total_chars / 4).max(1) as u32;
        }
        if prompt_tokens == 0 {
            let prompt_chars: usize = messages.iter().map(|m| m.content.len()).sum();
            prompt_tokens = (prompt_chars / 4).max(1) as u32;
        }

        let total_secs = total_duration.as_secs_f64();
        let tps = if total_secs > 0.0 && completion_tokens > 0 {
            completion_tokens as f64 / total_secs
        } else {
            0.0
        };

        let estimated_cost = self.calculate_cost(prompt_tokens, completion_tokens);

        Ok(ModelResponse {
            text: full_text,
            reasoning_content: if reasoning_text.is_empty() {
                None
            } else {
                Some(reasoning_text)
            },
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
            finish_reason: Some("stop".to_string()),
            raw_response: None,
        })
    }
}
