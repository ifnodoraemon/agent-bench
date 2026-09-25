use crate::config::ConfigFile;
use anyhow::{Context, Result};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Row, Table};
use eval_agent::{AgentAuditReport, VerificationAgent};
use eval_core::model::{create_client, ApiProtocol, ModelConfig};
use eval_core::verifier::{BaselineDriftReport, DriftStatus, ModelVerifier, VerificationReport};
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
    agentic: bool,
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

    let client = create_client(model_config.clone())
        .with_context(|| "Failed to create model client")?;

    if agentic {
        if !as_json {
            println!("\n🤖 正在启动由 LLM 驱动的自主智能验真 Agent 对抗审计 (Autonomous Agentic Audit)...");
            println!("  - 测试端点标识: {}", model_config.id);
            println!("  - 标称真实模型: {}", claimed_target_owned);
            println!("  - 审计机制: 现场动态合成对抗探针 + 多轮交叉审讯式反问 + 权威基线漂移校验");
            println!("  - 正在执行对抗审讯中，请稍候...\n");
        }

        let audit_report = VerificationAgent::audit(&*client, None, Some(&claimed_target_owned)).await?;

        if as_json {
            println!("{}", serde_json::to_string_pretty(&audit_report)?);
            return Ok(());
        }

        print_agent_audit_report(&audit_report);
        return Ok(());
    }

    if !as_json {
        println!("\n🔍 正在启动模型真实度与防伪嗅探 (Authenticity & Fingerprint Verification)...");
        println!("  - 测试端点标识: {}", model_config.id);
        println!("  - 标称真实模型: {}", claimed_target_owned);
        println!("  - API 协议类型: {:?}", model_config.protocol);
        println!("  - 探测探针规模: 6 项高敏旗舰能力与指纹探针");
        println!("  - 正在并发探测中，请稍候...\n");
    }

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

    // Table of baseline drift if available
    if let Some(ref drift) = report.baseline_drift {
        print_baseline_drift_table(drift);
    }

    println!("══════════════════════════════════════════════════════════════════════════════\n");
}

fn print_agent_audit_report(report: &AgentAuditReport) {
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("             🤖  AGENT-BENCH LLM-DRIVEN 自主智能验真 Agent 对抗审计凭证");
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("审计单号     : {}", report.audit_id);
    println!("目标端点模型 : {}", report.target_model);
    println!("对标权威基线 : {}", report.canonical_baseline_name);
    println!("审计生成时间 : {}", report.timestamp);
    println!("总计对抗耗时 : {} ms", report.total_latency_ms);
    println!("真实度得分   : {:.1} / 100.0", report.authenticity_score);
    println!("审计终局裁定 : {}", report.verdict_title);
    println!("阿谀附和指数 : {:.1}% ({})", report.sycophancy_index, report.sycophancy_assessment);

    println!("──────────────────────────────────────────────────────────────────────────────");
    println!("【Agent 裁决摘要】:\n{}", report.executive_summary);
    println!("──────────────────────────────────────────────────────────────────────────────");

    // Dynamic Probes Table
    let mut probe_table = Table::new();
    probe_table.load_preset(UTF8_FULL);
    probe_table.apply_modifier(UTF8_ROUND_CORNERS);
    probe_table.set_header(vec![
        Cell::new("动态对抗题目"),
        Cell::new("题型陷阱"),
        Cell::new("判定状态"),
        Cell::new("CoT思考链"),
        Cell::new("耗时"),
        Cell::new("审计点评"),
    ]);

    for p in &report.dynamic_probes {
        let status_cell = if p.passed {
            Cell::new("✓ 通过").fg(Color::Green)
        } else {
            Cell::new("✕ 踩雷").fg(Color::Red)
        };
        let cot_cell = if p.has_cot_signature {
            Cell::new("✓ 具备").fg(Color::Cyan)
        } else {
            Cell::new("-")
        };

        probe_table.add_row(Row::from(vec![
            Cell::new(&p.title),
            Cell::new(&p.trap_type),
            status_cell,
            cot_cell,
            Cell::new(format!("{}ms", p.response_latency_ms)),
            Cell::new(&p.auditor_critique),
        ]));
    }

    println!("\n【现场动态对抗探针结果】:");
    println!("{probe_table}");

    // Cross-examination dialogues
    if !report.cross_examinations.is_empty() {
        println!("\n【多轮交叉审讯式反问记录 (Cross-Examination Interrogation)】:");
        for (i, ce) in report.cross_examinations.iter().enumerate() {
            println!("  [第 {} 轮审讯: {}]", i + 1, ce.probe_title);
            println!("    - 初次答复截选: {}", ce.target_initial_answer);
            println!("    - 审讯员伪诱导: {}", ce.interrogator_challenge);
            println!("    - 模型辩护回复: {}", ce.target_defense_response.lines().next().unwrap_or(""));
            let tag = if ce.sycophancy_detected {
                "🚨 判定: 妥协盲从 (Sycophancy Trap Triggered)"
            } else {
                "🛡️ 判定: 坚定自洽 (Resisted Adversarial Pressure)"
            };
            println!("    - 审计点评: {} ({})\n", ce.auditor_notes, tag);
        }
    }

    // Baseline drift table
    if let Some(ref drift) = report.baseline_drift {
        print_baseline_drift_table(drift);
    }

    println!("══════════════════════════════════════════════════════════════════════════════\n");
}

fn print_baseline_drift_table(drift: &BaselineDriftReport) {
    if !drift.baseline_found || drift.comparisons.is_empty() {
        return;
    }

    println!("\n【权威榜单官方公布分与实测偏离度漂移分析 (Ground Truth Baselines)】:");
    println!("基准对标厂商 : {} | 论文/技术报告: {}", drift.vendor, drift.tech_report_url);
    println!("综合偏离漂移 : {:+.1}% | 裁决结论: {}", drift.mean_drift_pct, drift.verdict);

    let mut b_table = Table::new();
    b_table.load_preset(UTF8_FULL);
    b_table.apply_modifier(UTF8_ROUND_CORNERS);
    b_table.set_header(vec![
        Cell::new("权威评测集"),
        Cell::new("分类领域"),
        Cell::new("官方公布分"),
        Cell::new("实测得分"),
        Cell::new("偏离度 (Δ)"),
        Cell::new("基线吻合状态"),
    ]);

    for c in &drift.comparisons {
        let status_cell = match c.status {
            DriftStatus::Matching => Cell::new(c.status.display_badge()).fg(Color::Green),
            DriftStatus::MinorDeficit => Cell::new(c.status.display_badge()).fg(Color::Yellow),
            DriftStatus::SevereDeficit => Cell::new(c.status.display_badge()).fg(Color::Red),
            DriftStatus::SuspiciousOverfit => Cell::new(c.status.display_badge()).fg(Color::Magenta),
        };

        b_table.add_row(Row::from(vec![
            Cell::new(&c.benchmark_name),
            Cell::new(&c.category),
            Cell::new(format!("{:.1} {}", c.official_score, c.unit)),
            Cell::new(format!("{:.1} {}", c.tested_score, c.unit)),
            Cell::new(format!("{:+.1}%", c.delta_pct)),
            status_cell,
        ]));
    }

    println!("{b_table}");
}
