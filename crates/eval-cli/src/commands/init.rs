use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::Path;

pub fn execute_init(output_path: Option<String>) -> Result<()> {
    let target_file = output_path.unwrap_or_else(|| "eval_config.toml".to_string());

    println!("\n🚀 欢迎使用 agent-bench 交互式大模型评测配置向导！");
    println!("------------------------------------------------------------");

    // 1. Select Protocol Format
    println!("\n📌 [步骤 1/5] 请选择待测模型的底层 API 协议格式 (Protocol Format):");
    println!("  [1] OpenAI Chat Completions (/v1/chat/completions)");
    println!("      适用: 经典通用格式 (OpenAI, DeepSeek, 通义千问, 本地 Ollama/vLLM, Moonshot, Groq 等)");
    println!("  [2] OpenAI Responses API (/v1/responses)");
    println!("      适用: OpenAI 最新旗舰级 Responses 格式 (支持 input/instructions/内置搜索/思考流)");
    println!("  [3] Anthropic Claude (/v1/messages)");
    println!("      适用: Claude 3.5 Sonnet / Haiku / Opus 及 Claude 代理网关");
    println!("  [4] Google Gemini (generateContent)");
    println!("      适用: Google Gemini 1.5 / 2.0 原生协议端点与中转反代");
    println!("  [5] Mock Simulation (虚拟仿真模式，离线零成本评测)");

    let protocol_choice = prompt_input("请输入选项序号 [1-5] (默认 1): ", "1");
    let (protocol, default_url, default_model, default_key) = match protocol_choice.trim() {
        "2" => ("openai_response", "https://api.openai.com/v1", "gpt-4o", "env:OPENAI_API_KEY"),
        "3" => ("anthropic", "https://api.anthropic.com/v1", "claude-3-5-sonnet-20241022", "env:ANTHROPIC_API_KEY"),
        "4" => ("gemini", "https://generativelanguage.googleapis.com/v1beta", "gemini-2.0-flash", "env:GEMINI_API_KEY"),
        "5" => ("mock", "", "mock-model", "none"),
        _ => ("openai_chat", "https://api.deepseek.com/v1", "deepseek-chat", "env:DEEPSEEK_API_KEY"),
    };

    // 2. Model Identifier
    println!("\n📌 [步骤 2/5] 请输入模型的唯一标识 ID (用于生成报表与雷达图展示):");
    let model_id = prompt_input(&format!("模型 ID (默认: {}): ", default_model), default_model);

    // 3. Model Name
    println!("\n📌 [步骤 3/5] 请输入实际请求的 API 模型名称 (Model Name):");
    let model_name = prompt_input(&format!("Model Name (默认: {}): ", default_model), default_model);

    // 4. Base URL
    println!("\n📌 [步骤 4/5] 请输入接口 Base URL (支持自定义反代或本地服务):");
    let base_url = prompt_input(&format!("Base URL (默认: {}): ", default_url), default_url);

    // 5. API Key
    println!("\n📌 [步骤 5/5] 请输入 API Key 或环境变量引用 (例如 env:DEEPSEEK_API_KEY):");
    let api_key = prompt_input(&format!("API Key (默认: {}): ", default_key), default_key);

    // Concurrency
    let concurrency = prompt_input("评测并发任务数 Concurrency (默认: 5): ", "5");

    let config_content = format!(
r#"# agent-bench 评测配置文件 (由 agent-bench init 自动生成)

[benchmark]
concurrency = {concurrency}
output_dir = "./results"
save_json = true
save_markdown = true
save_html = true

[[models]]
id = "{model_id}"
protocol = "{protocol}"
model_name = "{model_name}"
base_url = "{base_url}"
api_key = "{api_key}"
temperature = 0.0
"#);

    if Path::new(&target_file).exists() {
        println!("\n⚠️ 文件 '{target_file}' 已存在，是否覆盖？[y/N]");
        let overwrite = prompt_input("", "n");
        if !overwrite.trim().eq_ignore_ascii_case("y") {
            println!("已取消操作，未写入文件。");
            return Ok(());
        }
    }

    std::fs::write(&target_file, config_content)
        .with_context(|| format!("无法写入配置文件: {target_file}"))?;

    println!("\n🎉 配置文件已成功生成: {}", target_file);
    println!("------------------------------------------------------------");
    println!("下一步运行指引:");
    println!("  1. 确保已在 .env 文件或终端环境变量中设置相应的 API Key");
    println!("  2. 运行评测命令:");
    println!("     cargo run --release --bin agent-bench -- run --config {target_file}\n");

    Ok(())
}

fn prompt_input(prompt_text: &str, default_value: &str) -> String {
    print!("{prompt_text}");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            default_value.to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        default_value.to_string()
    }
}
