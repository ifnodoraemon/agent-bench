use super::client::ModelClient;
use super::types::{
    ChatMessage, FunctionCall, ModelConfig, ModelResponse, TokenUsage, ToolCall, ToolDefinition,
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

        let start_time = Instant::now();
        let resp = self.send_request_with_retry(&body).await?;

        let mut stream = resp.bytes_stream().eventsource();
        let mut first_token_time: Option<Duration> = None;
        let mut full_text = String::new();
        let mut reasoning_text = String::new();
        let mut raw_tool_calls: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;

        while let Some(event_res) = stream.next().await {
            let event = event_res.context("Error in SSE stream event")?;
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
                    if let Some(delta) = choice.get("delta") {
                        if let Some(reasoning) = delta.get("reasoning_content").and_then(|r| r.as_str()) {
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

        Ok(ModelResponse {
            text: full_text,
            reasoning_content: if reasoning_text.is_empty() {
                None
            } else {
                Some(reasoning_text)
            },
            tool_calls: if final_tool_calls.is_empty() {
                None
            } else {
                Some(final_tool_calls)
            },
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
