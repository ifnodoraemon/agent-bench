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

pub struct GeminiClient {
    config: ModelConfig,
    http_client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl GeminiClient {
    pub fn new(config: ModelConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .unwrap_or_else(|| std::env::var("GEMINI_API_KEY").unwrap_or_default());

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

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
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());

        Ok(Self {
            config,
            http_client,
            api_key,
            base_url,
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

    async fn send_request_with_retry(&self, url: &str, body: &serde_json::Value) -> Result<reqwest::Response> {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = Duration::from_millis(500);

        loop {
            attempts += 1;
            let resp_result = self
                .http_client
                .post(url)
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
                        return Err(anyhow!("Gemini API error ({status}): {err_text}"));
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    if attempts < max_attempts {
                        tokio::time::sleep(delay).await;
                        delay *= 2;
                        continue;
                    }
                    return Err(anyhow!("Gemini network request failed after {attempts} attempts: {e}"));
                }
            }
        }
    }
}

#[async_trait]
impl ModelClient for GeminiClient {
    fn config(&self) -> &ModelConfig {
        &self.config
    }

    async fn chat_complete(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ModelResponse> {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for m in messages {
            match m.role {
                Role::System => {
                    system_instruction = Some(json!({
                        "parts": [{ "text": m.content }]
                    }));
                }
                Role::User => {
                    contents.push(json!({
                        "role": "user",
                        "parts": [{ "text": m.content }]
                    }));
                }
                Role::Assistant => {
                    let mut parts = Vec::new();
                    if !m.content.is_empty() {
                        parts.push(json!({ "text": m.content }));
                    }
                    if let Some(ref tcs) = m.tool_calls {
                        for tc in tcs {
                            let args_val: serde_json::Value =
                                serde_json::from_str(&tc.function.arguments)
                                    .unwrap_or(json!({}));
                            parts.push(json!({
                                "functionCall": {
                                    "name": tc.function.name,
                                    "args": args_val
                                }
                            }));
                        }
                    }
                    contents.push(json!({
                        "role": "model",
                        "parts": parts
                    }));
                }
                Role::Tool => {
                    let name = m.name.clone().unwrap_or_else(|| "tool_response".to_string());
                    contents.push(json!({
                        "role": "function",
                        "parts": [{
                            "functionResponse": {
                                "name": name,
                                "response": {
                                    "output": m.content
                                }
                            }
                        }]
                    }));
                }
            }
        }

        let mut body = json!({
            "contents": contents
        });

        if let Some(sys) = system_instruction {
            body["systemInstruction"] = sys;
        }

        let mut gen_config = json!({});
        if let Some(temp) = self.config.temperature {
            gen_config["temperature"] = json!(temp);
        }
        if let Some(max_t) = self.config.max_tokens {
            gen_config["maxOutputTokens"] = json!(max_t);
        }
        if let Some(top_p) = self.config.top_p {
            gen_config["topP"] = json!(top_p);
        }
        body["generationConfig"] = gen_config;

        if let Some(tool_list) = tools {
            if !tool_list.is_empty() {
                let function_declarations: Vec<serde_json::Value> = tool_list
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.function.name,
                            "description": t.function.description,
                            "parameters": t.function.parameters
                        })
                    })
                    .collect();
                body["tools"] = json!([{
                    "functionDeclarations": function_declarations
                }]);
            }
        }

        let model_id = &self.config.model_name;
        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url.trim_end_matches('/'),
            model_id,
            self.api_key
        );

        let start_time = Instant::now();
        let resp = self.send_request_with_retry(&url, &body).await?;

        let mut stream = resp.bytes_stream().eventsource();
        let mut first_token_time: Option<Duration> = None;
        let mut full_text = String::new();
        let mut tool_calls = Vec::new();

        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;

        while let Some(event_res) = stream.next().await {
            let event = event_res.context("Error in Gemini SSE stream")?;
            let parsed: serde_json::Value = match serde_json::from_str(&event.data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(usage) = parsed.get("usageMetadata") {
                if let Some(pt) = usage.get("promptTokenCount").and_then(|v| v.as_u64()) {
                    prompt_tokens = pt as u32;
                }
                if let Some(ct) = usage.get("candidatesTokenCount").and_then(|v| v.as_u64()) {
                    completion_tokens = ct as u32;
                }
            }

            if let Some(candidates) = parsed.get("candidates").and_then(|c| c.as_array()) {
                if let Some(candidate) = candidates.first() {
                    if let Some(content) = candidate.get("content") {
                        if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                            for part in parts {
                                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                    if first_token_time.is_none() {
                                        first_token_time = Some(start_time.elapsed());
                                    }
                                    full_text.push_str(text);
                                }
                                if let Some(fc) = part.get("functionCall") {
                                    if first_token_time.is_none() {
                                        first_token_time = Some(start_time.elapsed());
                                    }
                                    let fn_name = fc.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_string();
                                    let fn_args = fc.get("args").map(|a| a.to_string()).unwrap_or_else(|| "{}".to_string());
                                    tool_calls.push(ToolCall {
                                        id: format!("call_gemini_{}", tool_calls.len()),
                                        tool_type: "function".to_string(),
                                        function: FunctionCall {
                                            name: fn_name,
                                            arguments: fn_args,
                                        },
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let total_duration = start_time.elapsed();
        if completion_tokens == 0 && !full_text.is_empty() {
            completion_tokens = (full_text.len() / 4).max(1) as u32;
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
