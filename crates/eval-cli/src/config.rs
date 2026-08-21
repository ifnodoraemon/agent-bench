use anyhow::{Context, Result};
use eval_core::model::{ApiProtocol, ModelConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchGlobalConfig {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_true")]
    pub save_json: bool,
    #[serde(default = "default_true")]
    pub save_markdown: bool,
    #[serde(default = "default_true")]
    pub save_html: bool,
}

fn default_concurrency() -> usize {
    5
}
fn default_output_dir() -> String {
    "./results".to_string()
}
fn default_true() -> bool {
    true
}

impl Default for BenchGlobalConfig {
    fn default() -> Self {
        Self {
            concurrency: 5,
            output_dir: "./results".to_string(),
            save_json: true,
            save_markdown: true,
            save_html: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub protocol: Option<ApiProtocol>, // 显式协议: openai, anthropic, gemini, mock
    pub provider: Option<String>,      // 品牌名称 (可选，用于向前兼容)
    pub model_name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub extra_body: Option<HashMap<String, serde_json::Value>>,
    pub price_per_input_million: Option<f64>,
    pub price_per_output_million: Option<f64>,
}

impl ModelProfile {
    pub fn to_model_config(&self) -> ModelConfig {
        load_dotenv_if_exists();

        let resolved_api_key = self.api_key.as_ref().map(|k| {
            if k.starts_with("env:") {
                let var_name = &k[4..];
                std::env::var(var_name).unwrap_or_else(|_| k.clone())
            } else {
                k.clone()
            }
        });

        // Determine protocol
        let protocol = if let Some(proto) = self.protocol {
            proto
        } else if let Some(ref prov) = self.provider {
            ApiProtocol::from_str(prov).unwrap_or(ApiProtocol::OpenAiChat)
        } else {
            ApiProtocol::OpenAiChat
        };

        let provider_name = self.provider.clone().unwrap_or_else(|| protocol.to_string());

        ModelConfig {
            id: self.id.clone(),
            protocol,
            provider: provider_name,
            model_name: self.model_name.clone(),
            base_url: self.base_url.clone(),
            api_key: resolved_api_key,
            custom_headers: self.headers.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: self.top_p,
            response_format: None,
            reasoning_effort: self.reasoning_effort.clone(),
            extra_body: self.extra_body.clone(),
            price_per_input_million: self.price_per_input_million,
            price_per_output_million: self.price_per_output_million,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub benchmark: BenchGlobalConfig,
    pub judge: Option<ModelProfile>,
    #[serde(default)]
    pub models: Vec<ModelProfile>,
}

impl ConfigFile {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        load_dotenv_if_exists();
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        let config: ConfigFile = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;
        Ok(config)
    }
}

/// Automatically load environment variables from .env in the current working directory
pub fn load_dotenv_if_exists() {
    let dotenv_paths = [".env", "../.env"];
    for p in dotenv_paths {
        let path = Path::new(p);
        if path.exists() && path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let key = k.trim();
                        let val = v.trim().trim_matches('"').trim_matches('\'');
                        if std::env::var(key).is_err() {
                            std::env::set_var(key, val);
                        }
                    }
                }
            }
            break;
        }
    }
}
