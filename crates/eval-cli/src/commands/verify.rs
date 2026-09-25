use crate::config::ConfigFile;
use anyhow::{Context, Result};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Row, Table};
use eval_core::model::{create_client, ApiProtocol, ModelConfig};
use eval_core::verifier::{ModelVerifier, VerificationReport};
use std::path::Path;
use std::str::FromStr;

pub async fn execute_verify(
    model: Option<String>,
    target: Option<String>,
    config_path: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    protocol: Option<String>,
    as_json: bool,
) -> Result<()> {
    // 1. Resolve model config
    let model_id_or_name = model.unwrap_or_else(|| "mock-pro".to_string());
    let mut resolved_config = None;

    // Check config file first
    let cfg_to_try = config_path.as_deref().unwrap_or("agent_bench.toml");
    if Path::new(cfg_to_try).exists() {
        if let Ok(cfg_file) = ConfigFile::load_from_file(cfg_to_try) {
            if let Some(prof) = cfg_file.models.into_iter().find(|m| m.id == model_id_or_name || m.model_name == model_id_or_name) {
                resolved_config = Some(prof.to_model_config());
            }
        }
    }

    let mut model_config = match resolved_config {
        Some(c) => c,
        None => {
            // Build ad-hoc config
            let proto = if let Some(ref p_str) = protocol {
                ApiProtocol::from_str(p_str).unwrap_or(ApiProtocol::OpenAiChat)
            } else if model_id_or_name.starts_with("mock") {
                ApiProtocol::Mock
            } else if model_id_or_name.contains("claude") {
                ApiProtocol::Anthropic
            } else if model_id_or_name.contains("gemini") {
                ApiProtocol::Gemini
            } else {
                ApiProtocol::OpenAiChat
            };

            let mut cfg = ModelConfig::new(&model_id_or_name, proto.to_string(), &model_id_or_name);
            cfg.protocol = proto;
            cfg
        }
    };

    // Apply CLI overrides if present
    if let Some(url) = base_url {
        model_config.base_url = Some(url);
    }
    if let Some(key) = api_key {
        model_config.api_key = Some(key);
    }
    if let Some(p_str) = protocol {
        if let Ok(p) = ApiProtocol::from_str(&p_str) {
            model_config.protocol = p;
        }
    }

    let claimed_target_owned = target
        .unwrap_or_else(|| model_config.model_name.clone());

    if !as_json {
        println!("\n🔍 正在启动模型真实度与防伪嗅探 (Authenticity & Fingerprint Verification)...");
        println!("  - 测试端点标识: {}", model_config.id);
        println!("  - 标称真实模型: {}", claimed_target_owned);
        println!("  - API 协议类型: {:?}", model_config.protocol);
        println!("  - 探测探针规模: 6 项高敏旗舰能力与指纹探针");
        println!("  - 正在并发探测中，请稍候...\n");
    }

    let client = create_client(model_config)
        .with_context(|| "Failed to create model client")?;

    let report = ModelVerifier::verify(&*client, Some(&claimed_target_owned)).await?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    print_terminal_report(&report);
    Ok(())
}

fn print_terminal_report(report: &VerificationReport) {
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("                  🛡️  AGENT-BENCH 一键模型验真审计凭证报告");
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("测试模型标识 : {}", report.model_id);
    println!("标称真实模型 : {}", report.claimed_target);
    println!("验真完成时间 : {}", report.verified_at);
    println!("探测总计耗时 : {} ms", report.total_latency_ms);
    println!("真实度得分   : {:.1} / 100.0", report.authenticity_score);
    println!("验真最终结论 : {}", report.verdict_title);
    println!("估算量化级别 : {}", report.quantization_estimate);

    if !report.risk_tags.is_empty() {
        println!("潜在风险标记 : ⚠️  {}", report.risk_tags.join(" | "));
    }
    println!("──────────────────────────────────────────────────────────────────────────────");
    println!("【综合分析摘要】:\n{}", report.summary);
    println!("──────────────────────────────────────────────────────────────────────────────");

    // Table of probe results
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec![
        Cell::new("探针名称"),
        Cell::new("检测类型"),
        Cell::new("判定状态"),
        Cell::new("耗时"),
        Cell::new("指纹分析结论"),
    ]);

    for p in &report.probe_results {
        let status_cell = if p.passed {
            Cell::new("✓ 通过").fg(Color::Green)
        } else {
            Cell::new("✕ 未通过").fg(Color::Red)
        };

        table.add_row(Row::from(vec![
            Cell::new(&p.title),
            Cell::new(&p.category),
            status_cell,
            Cell::new(format!("{}ms", p.latency_ms)),
            Cell::new(&p.findings),
        ]));
    }

    println!("\n【各项高敏探针检测明细】:");
    println!("{table}");
    println!("══════════════════════════════════════════════════════════════════════════════\n");
}
