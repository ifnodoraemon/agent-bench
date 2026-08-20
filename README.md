# 🦀 agent-bench: 全维度大模型与 Agent 评测基准套件

`agent-bench` 是一个基于 **Rust** 开发的高性能、模块化的大语言模型与智能体综合能力评测系统。

## ✨ 核心特性

- **🎯 全维度评测覆盖**：
  - **基础能力 (Foundation)**：数学逻辑推理、IFEval 严格指令遵循、JSON Schema 格式校验、代码生成与单元测试沙箱执行。
  - **智能体能力 (Agentic & Tool Calling)**：单步/多步工具调用（Bash、REST API、SQL DB、Web 搜索）、ReAct 规划推理、环境交互与报错自我反思纠错。
  - **安全与鲁棒性 (Safety)**：Prompt 注入与越狱抵御、抗幻觉测试。
- **⚡ 高并发与精准度量**：
  - 基于 Tokio 异步多线程架构，流式（SSE）毫秒级捕获 **TTFT (Time to First Token)** 与 **TPS (Tokens/s)**。
  - 自动统计 Token 消耗量及单次/总体调用成本 ($ USD)。
- **⚖️ 丰富判定引擎**：
  - 支持精确匹配、正则表达式、JSON Schema 校验、Python/Rust 代码执行沙箱。
  - 支持 LLM-as-a-Judge 自动对齐判定与位置偏见消除。
  - 支持 Pairwise 对抗与 **Elo 竞技场积分** 计算。
- **📊 多格式报告输出**：
  - 终端彩色表格、GitHub Flavored Markdown 评测报告、交互式 HTML 可视化看板。

---

## 🚀 快速上手

### 1. 编译构建
```bash
cargo build --release
```

### 2. 验证评测数据集
```bash
cargo run --bin agent-bench -- validate datasets/foundation/*.jsonl datasets/agent/*.jsonl
```

### 3. 运行评测
```bash
# 使用配置文件运行多模型评测
cargo run --bin agent-bench -- run --config eval_config.toml

# 过滤特定类别 (如仅评测 agent 智能体能力)
cargo run --bin agent-bench -- run --category agent --concurrency 5

# 指定评测特定模型
cargo run --bin agent-bench -- run --models deepseek-chat,gpt-4o-mini
```

### 4. 历史结果对比与 Elo 排行榜
```bash
cargo run --bin agent-bench -- compare results/eval_results_*.json
```

---

## 📁 目录结构

```text
agent-bench/
├── Cargo.toml
├── crates/
│   ├── eval-core/        # 统一 Provider 适配器、数据加载、判定引擎、指标统计、报告生成
│   ├── eval-agent/       # Agent 模拟交互环境 (Bash/Web/DB)、多轮 Trajectory 追踪与判定
│   ├── eval-suites/      # 评测套件编排器 (并发调度、Judge 分流)
│   └── eval-cli/         # 命令行界面与配置文件解析
├── datasets/             # 内置各维度测试用例 (JSONL)
└── eval_config.example.toml
```
