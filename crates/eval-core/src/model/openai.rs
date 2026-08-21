use super::client::ModelClient;
use super::types::{
    extract_fallback_tool_calls, ChatMessage, FunctionCall, ModelConfig, ModelResponse, TokenUsage,
    ToolCall, ToolDefinition,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct OpenAICompatibleClient {
    config: ModelConfig,
    http_client: reqwest::Client,
    endpoint: String,
}

impl OpenAICompatibleClient {
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

        // Inject custom headers if provided
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
        let endpoint = if base_url.ends_with("/chat/completions") {
            base_url
        } else {
            format!("{}/chat/completions", base_url.trim_end_matches('/'))
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
                        return Err(anyhow!("OpenAI API error ({status}): {err_text}"));
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
impl ModelClient for OpenAICompatibleClient {
    fn config(&self) -> &ModelConfig {
        &self.config
    }

    async fn chat_complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ModelResponse> {
        let mut body = json!({
            "model": self.config.model_name,
            "messages": messages,
            "stream": true,
            "stream_options": { "include_usage": true }
        });

        if let Some(temp) = self.config.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(top_p) = self.config.top_p {
            body["top_p"] = json!(top_p);
        }
        if let Some(ref effort) = self.config.reasoning_effort {
            body["reasoning_effort"] = json!(effort);
        }
        if let Some(ref extra) = self.config.extra_body {
            if let Some(map) = body.as_object_mut() {
                for (k, v) in extra {
                    map.insert(k.clone(), v.clone());
                }
            }
        }
        if let Some(ref rf) = self.config.response_format {
            if let Some(ref schema) = rf.json_schema {
                body["response_format"] = json!({
                    "type": "json_schema",
                    "json_schema": schema
                });
            } else {
                body["response_format"] = json!({
                    "type": rf.format_type
                });
            }
        }
        if let Some(tool_list) = tools {
            if !tool_list.is_empty() {
                body["tools"] = json!(tool_list);
            }
        }

        let mut current_max_tokens = self.config.max_tokens.unwrap_or(4096);
        body["max_tokens"] = json!(current_max_tokens);

        let mut attempts = 0;
        let max_attempts = 3;
        let mut retry_delay = Duration::from_millis(800);

        loop {
            attempts += 1;
            let start_time = Instant::now();
            let req_res = self.execute_streaming_request(&body, messages, start_time).await;

            match req_res {
                Ok(resp) => {
                    if resp.finish_reason.as_deref() == Some("length") && attempts < max_attempts {
                        current_max_tokens = (current_max_tokens * 2).min(16384);
                        body["max_tokens"] = json!(current_max_tokens);
                        tracing::warn!(
                            "Response truncated (finish_reason=length) on attempt {attempts}, dynamically expanding max_tokens to {current_max_tokens} and retrying..."
                        );
                        tokio::time::sleep(retry_delay).await;
                        retry_delay *= 2;
                        continue;
                    }

                    if resp.text.trim().is_empty() && resp.tool_calls.is_none() && attempts < max_attempts {
                        tracing::warn!("Model returned empty output on attempt {attempts}, retrying in {retry_delay:?}...");
                        tokio::time::sleep(retry_delay).await;
                        retry_delay *= 2;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    if attempts < max_attempts {
                        tracing::warn!("Streaming request failed on attempt {attempts} ({e}), retrying in {retry_delay:?}...");
                        tokio::time::sleep(retry_delay).await;
                        retry_delay *= 2;
                        continue;
                    }

                    // Fallback to robust non-streaming HTTP POST request as ultimate safety net
                    tracing::warn!("All {max_attempts} streaming attempts failed ({e}). Falling back to non-streaming request...");
                    let start_time = Instant::now();
                    return self.execute_non_streaming_request(&body, messages, start_time).await;
                }
            }
        }
    }
}

impl OpenAICompatibleClient {
    async fn execute_streaming_request(
        &self,
        body: &serde_json::Value,
        messages: &[ChatMessage],
        start_time: Instant,
    ) -> Result<ModelResponse> {
        let resp = self.send_request_with_retry(body).await?;

        let mut stream = resp.bytes_stream().eventsource();
        let mut first_token_time: Option<Duration> = None;
        let mut full_text = String::new();
        let mut reasoning_text = String::new();
        let mut raw_tool_calls: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut finish_reason: Option<String> = None;
        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;

        loop {
            let next_event = tokio::time::timeout(Duration::from_secs(35), stream.next()).await;
            let event_res = match next_event {
                Ok(Some(ev_res)) => ev_res,
                Ok(None) => break,
                Err(_) => {
                    if !full_text.is_empty() || !reasoning_text.is_empty() || !raw_tool_calls.is_empty() {
                        tracing::info!("SSE stream chunk timed out after 35s. Salvaging accumulated response.");
                        break;
                    } else {
                        return Err(anyhow::anyhow!("SSE stream timed out waiting for chunk from server (exceeded 35s)"));
                    }
                }
            };

            let event = match event_res {
                Ok(ev) => ev,
                Err(err) => {
                    // Gracefully salvage content if server disconnected after outputting response
                    if !full_text.is_empty() || !reasoning_text.is_empty() || !raw_tool_calls.is_empty() {
                        tracing::info!("Gracefully salvaging accumulated response despite SSE stream termination: {err}");
                        break;
                    } else {
                        return Err(anyhow::anyhow!("SSE stream interrupted before receiving any content: {err}"));
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

            if let Some(usage) = parsed.get("usage") {
                if let Some(pt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                    prompt_tokens = pt as u32;
                }
                if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
                    completion_tokens = ct as u32;
                }
            }

            if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                if let Some(choice) = choices.first() {
                    if let Some(fr) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                        finish_reason = Some(fr.to_string());
                    }

                    if let Some(delta) = choice.get("delta") {
                        let reasoning_opt = delta
                            .get("reasoning_content")
                            .or_else(|| delta.get("reasoning"))
                            .and_then(|r| r.as_str());

                        if let Some(reasoning) = reasoning_opt {
                            if !reasoning.is_empty() {
                                if first_token_time.is_none() {
                                    first_token_time = Some(start_time.elapsed());
                                }
                                reasoning_text.push_str(reasoning);
                            }
                        }

                        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                            if !content.is_empty() {
                                if first_token_time.is_none() {
                                    first_token_time = Some(start_time.elapsed());
                                }
                                full_text.push_str(content);
                            }
                        }

                        if let Some(t_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                            if first_token_time.is_none() {
                                first_token_time = Some(start_time.elapsed());
                            }
                            for tc in t_calls {
                                let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                let entry = raw_tool_calls.entry(index).or_insert_with(|| (
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                ));

                                if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                    entry.0.push_str(id);
                                }
                                if let Some(func) = tc.get("function") {
                                    if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                        entry.1.push_str(name);
                                    }
                                    if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                        entry.2.push_str(args);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let total_duration = start_time.elapsed();

        let mut final_tool_calls = Vec::new();
        let mut sorted_indices: Vec<_> = raw_tool_calls.keys().cloned().collect();
        sorted_indices.sort_unstable();
        for idx in sorted_indices {
            if let Some((id, name, args)) = raw_tool_calls.remove(&idx) {
                final_tool_calls.push(ToolCall {
                    id: if id.is_empty() { format!("call_{idx}") } else { id },
                    tool_type: "function".to_string(),
                    function: FunctionCall {
                        name,
                        arguments: args,
                    },
                });
            }
        }

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

        if full_text.trim().is_empty() && !reasoning_text.trim().is_empty() {
            full_text = reasoning_text.clone();
        }

        let fallback_tools = if !final_tool_calls.is_empty() {
            Some(final_tool_calls)
        } else {
            extract_fallback_tool_calls(&full_text)
        };

        Ok(ModelResponse {
            text: full_text,
            reasoning_content: if reasoning_text.is_empty() {
                None
            } else {
                Some(reasoning_text)
            },
            tool_calls: fallback_tools,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            total_duration,
            ttft: first_token_time,
            tokens_per_second: tps,
            estimated_cost_usd: estimated_cost,
            finish_reason,
            raw_response: None,
        })
    }

    async fn execute_non_streaming_request(
        &self,
        body: &serde_json::Value,
        _messages: &[ChatMessage],
        start_time: Instant,
    ) -> Result<ModelResponse> {
        let mut non_stream_body = body.clone();
        non_stream_body["stream"] = serde_json::json!(false);

        let resp = self.send_request_with_retry(&non_stream_body).await?;
        let json_resp: serde_json::Value = resp.json().await?;

        let choice = json_resp
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("No choices returned in non-streaming response"))?;

        let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());

        let message = choice.get("message").ok_or_else(|| anyhow::anyhow!("No message object in choice"))?;
        let mut text = message.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
        let reasoning_content = message
            .get("reasoning_content")
            .or_else(|| message.get("reasoning"))
            .and_then(|r| r.as_str())
            .map(|s| s.to_string());

        if text.trim().is_empty() {
            if let Some(ref r) = reasoning_content {
                text = r.clone();
            }
        }

        let mut final_tool_calls = Vec::new();
        if let Some(tcs) = message.get("tool_calls").and_then(|t| t.as_array()) {
            for (idx, tc) in tcs.iter().enumerate() {
                let id = tc.get("id").and_then(|i| i.as_str()).unwrap_or(&format!("call_{idx}")).to_string();
                let name = tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()).unwrap_or("").to_string();
                let arguments = tc.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()).unwrap_or("").to_string();
                final_tool_calls.push(ToolCall {
                    id,
                    tool_type: "function".to_string(),
                    function: FunctionCall { name, arguments },
                });
            }
        }

        let usage = json_resp.get("usage");
        let prompt_tokens = usage.and_then(|u| u.get("prompt_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let completion_tokens = usage.and_then(|u| u.get("completion_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let total_duration = start_time.elapsed();
        let total_secs = total_duration.as_secs_f64();
        let tps = if total_secs > 0.0 && completion_tokens > 0 {
            completion_tokens as f64 / total_secs
        } else {
            0.0
        };

        let fallback_tools = if !final_tool_calls.is_empty() {
            Some(final_tool_calls)
        } else {
            extract_fallback_tool_calls(&text)
        };

        let estimated_cost = self.calculate_cost(prompt_tokens, completion_tokens);

        Ok(ModelResponse {
            text,
            reasoning_content,
            tool_calls: fallback_tools,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            total_duration,
            ttft: None,
            tokens_per_second: tps,
            estimated_cost_usd: estimated_cost,
            finish_reason,
            raw_response: None,
        })
    }
}
