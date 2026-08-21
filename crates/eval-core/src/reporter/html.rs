use crate::metrics::ModelBenchmarkSummary;
use chrono::Local;

pub struct HtmlReporter;

impl HtmlReporter {
    pub fn generate_html(summaries: &[ModelBenchmarkSummary]) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let summaries_json = serde_json::to_string(summaries).unwrap_or_else(|_| "[]".to_string());

        let mut sorted_summaries = summaries.to_vec();
        sorted_summaries.sort_by(|a, b| {
            b.overall_accuracy
                .partial_cmp(&a.overall_accuracy)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_models = sorted_summaries.len();
        let best_model = sorted_summaries.first();
        let best_model_name = best_model.map(|s| s.model_name.as_str()).unwrap_or("-");
        let best_accuracy = best_model
            .map(|s| format!("{:.1}%", s.overall_accuracy * 100.0))
            .unwrap_or_else(|| "-".to_string());

        let fastest_ttft = sorted_summaries
            .iter()
            .filter_map(|s| s.avg_ttft_ms)
            .fold(f64::INFINITY, f64::min);
        let fastest_ttft_str = if fastest_ttft.is_finite() {
            format!("{:.0}ms", fastest_ttft)
        } else {
            "-".to_string()
        };

        let max_tps = sorted_summaries
            .iter()
            .map(|s| s.avg_tps)
            .fold(0.0f64, f64::max);
        let max_tps_str = if max_tps > 0.0 {
            format!("{:.1} tok/s", max_tps)
        } else {
            "-".to_string()
        };

        let table_rows = sorted_summaries
            .iter()
            .enumerate()
            .map(|(rank, s)| {
                let rank_badge = match rank {
                    0 => r#"<span class="rank-badge rank-1">🥇 1</span>"#,
                    1 => r#"<span class="rank-badge rank-2">🥈 2</span>"#,
                    2 => r#"<span class="rank-badge rank-3">🥉 3</span>"#,
                    _ => &format!(r#"<span class="rank-badge rank-n">{}</span>"#, rank + 1),
                };

                let ttft = s
                    .avg_ttft_ms
                    .map(|t| format!("{:.0}ms", t))
                    .unwrap_or_else(|| "-".to_string());

                let acc_pct = s.overall_accuracy * 100.0;
                let acc_class = if acc_pct >= 85.0 {
                    "badge-success"
                } else if acc_pct >= 60.0 {
                    "badge-warning"
                } else {
                    "badge-danger"
                };

                let bar_color = if acc_pct >= 85.0 {
                    "var(--accent-emerald)"
                } else if acc_pct >= 60.0 {
                    "var(--accent-amber)"
                } else {
                    "var(--accent-rose)"
                };

                format!(
                    r#"<tr>
                    <td class="text-center">{}</td>
                    <td><div class="model-name-cell"><strong>{}</strong><span class="model-id">{}</span></div></td>
                    <td>
                        <div class="acc-cell">
                            <span class="badge {}">{:.1}%</span>
                            <span class="acc-count">({}/{})</span>
                            <div class="mini-progress-bg"><div class="mini-progress-fill" style="width: {:.1}%; background: {};"></div></div>
                        </div>
                    </td>
                    <td><span class="score-pill">{:.2}</span></td>
                    <td>{:.0}ms</td>
                    <td>{:.0}ms</td>
                    <td><span class="ttft-badge">{}</span></td>
                    <td><strong>{:.1}</strong></td>
                    <td><span class="token-in">{}</span> / <span class="token-out">{}</span></td>
                    <td>${:.5}</td>
                </tr>"#,
                    rank_badge,
                    s.model_name,
                    s.model_id,
                    acc_class,
                    acc_pct,
                    s.passed_cases,
                    s.total_cases,
                    acc_pct,
                    bar_color,
                    s.overall_score,
                    s.avg_latency_ms,
                    s.p95_latency_ms,
                    ttft,
                    s.avg_tps,
                    s.total_prompt_tokens,
                    s.total_completion_tokens,
                    s.total_cost_usd
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        HTML_TEMPLATE
            .replace("__NOW__", &now)
            .replace("__TOTAL_MODELS__", &total_models.to_string())
            .replace("__BEST_MODEL_NAME__", best_model_name)
            .replace("__BEST_ACCURACY__", &best_accuracy)
            .replace("__FASTEST_TTFT__", &fastest_ttft_str)
            .replace("__MAX_TPS__", &max_tps_str)
            .replace("__TABLE_ROWS__", &table_rows)
            .replace("__SUMMARIES_JSON__", &summaries_json)
    }
}

const HTML_TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="zh-CN" data-theme="warm">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>agent-bench 全维度评测大屏</title>
    <!-- Google Fonts -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
    <!-- Chart.js CDN -->
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        /* Theme: Pro Studio Light (Linear / Vercel Neutral Minimalist) */
        :root, html[data-theme="warm"] {
            --bg-body: #f8fafc;
            --bg-surface: #ffffff;
            --bg-subtle: #f1f5f9;
            --bg-hover: #e2e8f0;
            --border-soft: #e2e8f0;
            --border-strong: #cbd5e1;
            --text-heading: #0f172a;
            --text-body: #334155;
            --text-muted: #64748b;
            --text-faint: #94a3b8;
            --primary: #4f46e5;
            --primary-bg: rgba(79, 70, 229, 0.08);
            --primary-border: rgba(79, 70, 229, 0.22);
            --accent-emerald: #16a34a;
            --emerald-bg: rgba(22, 163, 74, 0.08);
            --emerald-border: rgba(22, 163, 74, 0.22);
            --accent-amber: #d97706;
            --amber-bg: rgba(217, 119, 6, 0.08);
            --amber-border: rgba(217, 119, 6, 0.22);
            --accent-rose: #dc2626;
            --rose-bg: rgba(220, 38, 38, 0.08);
            --rose-border: rgba(220, 38, 38, 0.22);
            --accent-sky: #0284c7;
            --sky-bg: rgba(2, 132, 199, 0.08);
            --sky-border: rgba(2, 132, 199, 0.22);
            --shadow-sm: 0 1px 2px 0 rgba(15, 23, 42, 0.03);
            --shadow-card: 0 1px 3px 0 rgba(15, 23, 42, 0.04), 0 1px 2px -1px rgba(15, 23, 42, 0.02);
            --shadow-hover: 0 4px 6px -1px rgba(15, 23, 42, 0.06), 0 2px 4px -2px rgba(15, 23, 42, 0.03);
            --code-bg: #f8fafc;
            --code-border: #e2e8f0;
            --rank-1-bg: rgba(234, 179, 8, 0.12);
            --rank-1-text: #a16207;
            --rank-1-border: rgba(234, 179, 8, 0.3);
            --rank-2-bg: rgba(100, 116, 139, 0.1);
            --rank-2-text: #475569;
            --rank-2-border: rgba(100, 116, 139, 0.25);
            --rank-3-bg: rgba(234, 88, 12, 0.1);
            --rank-3-text: #c2410c;
            --rank-3-border: rgba(234, 88, 12, 0.25);
        }

        /* Theme: Pro Carbon Dark (Linear / Vercel Neutral Minimalist) */
        html[data-theme="dark"] {
            --bg-body: #09090b;
            --bg-surface: #121215;
            --bg-subtle: #18181b;
            --bg-hover: #202024;
            --border-soft: #27272a;
            --border-strong: #3f3f46;
            --text-heading: #fafafa;
            --text-body: #d4d4d8;
            --text-muted: #a1a1aa;
            --text-faint: #71717a;
            --primary: #6366f1;
            --primary-bg: rgba(99, 102, 241, 0.14);
            --primary-border: rgba(99, 102, 241, 0.32);
            --accent-emerald: #22c55e;
            --emerald-bg: rgba(34, 197, 94, 0.12);
            --emerald-border: rgba(34, 197, 94, 0.25);
            --accent-amber: #eab308;
            --amber-bg: rgba(234, 179, 8, 0.12);
            --amber-border: rgba(234, 179, 8, 0.25);
            --accent-rose: #ef4444;
            --rose-bg: rgba(239, 68, 68, 0.12);
            --rose-border: rgba(239, 68, 68, 0.25);
            --accent-sky: #38bdf8;
            --sky-bg: rgba(56, 189, 248, 0.12);
            --sky-border: rgba(56, 189, 248, 0.25);
            --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.4);
            --shadow-card: 0 1px 3px 0 rgba(0, 0, 0, 0.4), 0 1px 2px -1px rgba(0, 0, 0, 0.3);
            --shadow-hover: 0 4px 10px -1px rgba(0, 0, 0, 0.6);
            --code-bg: #09090b;
            --code-border: #27272a;
            --rank-1-bg: rgba(234, 179, 8, 0.16);
            --rank-1-text: #fde047;
            --rank-1-border: rgba(234, 179, 8, 0.3);
            --rank-2-bg: rgba(161, 161, 170, 0.14);
            --rank-2-text: #e4e4e7;
            --rank-2-border: rgba(161, 161, 170, 0.25);
            --rank-3-bg: rgba(249, 115, 22, 0.16);
            --rank-3-text: #fdba74;
            --rank-3-border: rgba(249, 115, 22, 0.3);
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: 'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background-color: var(--bg-body);
            color: var(--text-body);
            padding: 2rem 1.5rem;
            line-height: 1.55;
            -webkit-font-smoothing: antialiased;
            transition: background-color 0.2s ease, color 0.2s ease;
        }
        .container {
            max-width: 1400px;
            margin: 0 auto;
        }

        /* Header */
        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            background: var(--bg-surface);
            padding: 1.35rem 1.75rem;
            border-radius: 12px;
            border: 1px solid var(--border-soft);
            box-shadow: var(--shadow-card);
            margin-bottom: 1.75rem;
            flex-wrap: wrap;
            gap: 1rem;
        }
        .header-title-group h1 {
            font-size: 1.55rem;
            font-weight: 800;
            color: var(--text-heading);
            display: flex;
            align-items: center;
            gap: 0.6rem;
            letter-spacing: -0.025em;
        }
        .header-title-group .subtitle {
            color: var(--text-muted);
            font-size: 0.88rem;
            margin-top: 0.25rem;
            font-weight: 500;
        }
        .header-actions {
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }
        .time-badge {
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            background: var(--bg-subtle);
            padding: 0.4rem 0.85rem;
            border-radius: 8px;
            font-size: 0.82rem;
            color: var(--text-muted);
            font-weight: 600;
            border: 1px solid var(--border-soft);
        }
        .theme-toggle-btn {
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            background: var(--bg-subtle);
            border: 1px solid var(--border-soft);
            color: var(--text-body);
            padding: 0.4rem 0.85rem;
            border-radius: 8px;
            cursor: pointer;
            font-size: 0.82rem;
            font-weight: 600;
            transition: all 0.15s;
        }
        .theme-toggle-btn:hover {
            background: var(--bg-hover);
            border-color: var(--border-strong);
        }

        /* KPI Cards Grid */
        .kpi-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
            gap: 1.25rem;
            margin-bottom: 1.75rem;
        }
        .kpi-card {
            background: var(--bg-surface);
            border: 1px solid var(--border-soft);
            border-radius: 12px;
            padding: 1.25rem 1.4rem;
            box-shadow: var(--shadow-card);
            transition: transform 0.2s, box-shadow 0.2s;
            position: relative;
            overflow: hidden;
        }
        .kpi-card::before {
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 3px;
            background: var(--primary);
            opacity: 0.8;
        }
        .kpi-card:hover {
            transform: translateY(-2px);
            box-shadow: var(--shadow-hover);
        }
        .kpi-label {
            font-size: 0.8rem;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--text-faint);
            margin-bottom: 0.35rem;
        }
        .kpi-value {
            font-size: 1.85rem;
            font-weight: 800;
            color: var(--text-heading);
            letter-spacing: -0.03em;
        }
        .kpi-subtext {
            font-size: 0.82rem;
            color: var(--text-muted);
            margin-top: 0.25rem;
        }
        .kpi-icon {
            float: right;
            font-size: 1.6rem;
            opacity: 0.8;
        }

        /* Card Section */
        .card {
            background: var(--bg-surface);
            border-radius: 12px;
            padding: 1.5rem;
            border: 1px solid var(--border-soft);
            box-shadow: var(--shadow-card);
            margin-bottom: 1.75rem;
        }
        .card-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 1.25rem;
            padding-bottom: 0.85rem;
            border-bottom: 1px solid var(--border-soft);
        }
        .card-title {
            font-size: 1.12rem;
            font-weight: 700;
            color: var(--text-heading);
            display: flex;
            align-items: center;
            gap: 0.55rem;
            letter-spacing: -0.015em;
        }

        /* Grid for Charts */
        .grid-2 {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(560px, 1fr));
            gap: 1.5rem;
            margin-bottom: 1.75rem;
        }
        .chart-container {
            position: relative;
            height: 320px;
            width: 100%;
        }

        /* Table */
        .table-responsive {
            overflow-x: auto;
        }
        table {
            width: 100%;
            border-collapse: separate;
            border-spacing: 0;
            text-align: left;
            font-size: 0.88rem;
        }
        th {
            background-color: var(--bg-subtle);
            color: var(--text-muted);
            font-weight: 700;
            font-size: 0.75rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            padding: 0.85rem 1rem;
            border-top: 1px solid var(--border-soft);
            border-bottom: 1px solid var(--border-strong);
        }
        td {
            padding: 0.9rem 1rem;
            border-bottom: 1px solid var(--border-soft);
            color: var(--text-body);
            vertical-align: middle;
        }
        tr:hover td {
            background-color: var(--bg-hover);
        }
        .text-center { text-align: center; }

        .model-name-cell strong {
            display: block;
            color: var(--text-heading);
            font-size: 0.95rem;
            font-weight: 700;
        }
        .model-id {
            font-size: 0.75rem;
            color: var(--text-faint);
            font-family: 'JetBrains Mono', monospace;
        }

        .rank-badge {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            font-weight: 700;
            font-size: 0.82rem;
            padding: 0.2rem 0.55rem;
            border-radius: 9999px;
        }
        .rank-1 { background: var(--rank-1-bg); color: var(--rank-1-text); border: 1px solid var(--rank-1-border); }
        .rank-2 { background: var(--rank-2-bg); color: var(--rank-2-text); border: 1px solid var(--rank-2-border); }
        .rank-3 { background: var(--rank-3-bg); color: var(--rank-3-text); border: 1px solid var(--rank-3-border); }
        .rank-n { color: var(--text-faint); }

        .acc-cell {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            flex-wrap: wrap;
        }
        .acc-count {
            font-size: 0.8rem;
            color: var(--text-muted);
            font-weight: 500;
        }
        .mini-progress-bg {
            width: 100%;
            height: 5px;
            background: var(--border-soft);
            border-radius: 999px;
            overflow: hidden;
            margin-top: 0.25rem;
        }
        .mini-progress-fill {
            height: 100%;
            border-radius: 999px;
        }

        .score-pill {
            font-weight: 700;
            color: var(--text-heading);
            background: var(--bg-subtle);
            padding: 0.2rem 0.5rem;
            border-radius: 6px;
            border: 1px solid var(--border-soft);
        }
        .ttft-badge {
            color: var(--accent-emerald);
            font-weight: 600;
        }
        .token-in { color: var(--accent-sky); font-weight: 600; }
        .token-out { color: var(--accent-emerald); font-weight: 600; }

        .badge {
            display: inline-block;
            padding: 0.2rem 0.55rem;
            border-radius: 6px;
            font-weight: 700;
            font-size: 0.82rem;
            letter-spacing: -0.01em;
        }
        .badge-success { background: var(--emerald-bg); color: var(--accent-emerald); border: 1px solid var(--emerald-border); }
        .badge-warning { background: var(--amber-bg); color: var(--accent-amber); border: 1px solid var(--amber-border); }
        .badge-danger { background: var(--rose-bg); color: var(--accent-rose); border: 1px solid var(--rose-border); }

        /* Case Inspector */
        .controls-bar {
            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 1rem;
            margin-bottom: 1.25rem;
            flex-wrap: wrap;
        }
        .tab-buttons {
            display: flex;
            gap: 0.4rem;
        }
        .tab-btn {
            background: var(--bg-subtle);
            border: 1px solid var(--border-soft);
            color: var(--text-muted);
            padding: 0.45rem 1.1rem;
            border-radius: 8px;
            cursor: pointer;
            font-size: 0.85rem;
            font-weight: 600;
            transition: all 0.15s ease-in-out;
        }
        .tab-btn:hover {
            background: var(--bg-hover);
            color: var(--text-heading);
        }
        .tab-btn.active {
            background: var(--primary);
            color: #ffffff;
            border-color: var(--primary);
            box-shadow: 0 1px 3px rgba(67, 56, 202, 0.3);
        }

        .filter-group {
            display: flex;
            align-items: center;
            gap: 0.4rem;
        }
        .search-input {
            padding: 0.45rem 0.85rem;
            border: 1px solid var(--border-soft);
            border-radius: 8px;
            font-size: 0.85rem;
            background: var(--bg-surface);
            color: var(--text-heading);
            outline: none;
            width: 240px;
            transition: border-color 0.2s, box-shadow 0.2s;
        }
        .search-input:focus {
            border-color: var(--primary);
            box-shadow: 0 0 0 3px var(--primary-bg);
        }
        .filter-btn {
            padding: 0.45rem 0.8rem;
            border: 1px solid var(--border-soft);
            border-radius: 8px;
            background: var(--bg-surface);
            font-size: 0.82rem;
            font-weight: 600;
            cursor: pointer;
            color: var(--text-muted);
            transition: all 0.15s;
        }
        .filter-btn.active {
            background: var(--bg-subtle);
            color: var(--text-heading);
            border-color: var(--border-strong);
        }

        .case-item {
            background: var(--bg-surface);
            border: 1px solid var(--border-soft);
            border-radius: 10px;
            margin-bottom: 0.75rem;
            overflow: hidden;
            box-shadow: var(--shadow-sm);
            transition: border-color 0.15s, box-shadow 0.15s;
        }
        .case-item:hover {
            border-color: var(--border-strong);
            box-shadow: var(--shadow-card);
        }
        .case-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 0.85rem 1.15rem;
            cursor: pointer;
            font-weight: 600;
            background: var(--bg-surface);
            user-select: none;
        }
        .case-header:hover {
            background: var(--bg-hover);
        }
        .case-category-tag {
            display: inline-block;
            background: var(--bg-subtle);
            color: var(--text-muted);
            font-size: 0.74rem;
            padding: 0.15rem 0.5rem;
            border-radius: 4px;
            margin: 0 0.5rem;
            font-weight: 700;
            letter-spacing: 0.03em;
            border: 1px solid var(--border-soft);
        }
        .case-meta {
            display: flex;
            align-items: center;
            gap: 1.2rem;
            color: var(--text-muted);
            font-size: 0.82rem;
        }
        .case-body {
            padding: 1.15rem;
            background: var(--bg-subtle);
            border-top: 1px solid var(--border-soft);
            font-size: 0.86rem;
            color: var(--text-body);
        }
        .case-section {
            margin-bottom: 0.9rem;
        }
        .case-section:last-child {
            margin-bottom: 0;
        }
        .section-label {
            font-size: 0.75rem;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--text-faint);
            margin-bottom: 0.35rem;
        }
        .code-box {
            background: var(--code-bg);
            border: 1px solid var(--code-border);
            border-radius: 8px;
            padding: 0.85rem 1rem;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.83rem;
            white-space: pre-wrap;
            word-break: break-all;
            color: var(--text-heading);
            max-height: 280px;
            overflow-y: auto;
            line-height: 1.5;
        }

        footer {
            text-align: center;
            color: var(--text-faint);
            font-size: 0.82rem;
            margin-top: 3rem;
            padding-top: 1.5rem;
            border-top: 1px solid var(--border-soft);
        }
    </style>
</head>
<body>
    <div class="container">
        <!-- Header -->
        <header>
            <div class="header-title-group">
                <h1>⚡ agent-bench 综合能力评测大屏</h1>
                <div class="subtitle">大语言模型与 Agent 智能体全维度基准评测 • 基础推理 • 工具调用 • 安全合规 • 吞吐与成本</div>
            </div>
            <div class="header-actions">
                <div class="time-badge">🕒 __NOW__</div>
                <button class="theme-toggle-btn" onclick="toggleTheme()">🌓 切换主题</button>
            </div>
        </header>

        <!-- KPI Metric Cards -->
        <div class="kpi-grid">
            <div class="kpi-card">
                <div class="kpi-icon">🤖</div>
                <div class="kpi-label">已评测模型总数</div>
                <div class="kpi-value">__TOTAL_MODELS__</div>
                <div class="kpi-subtext">同台标准化基准对齐</div>
            </div>
            <div class="kpi-card" style="border-top-color: var(--primary);">
                <div class="kpi-icon">👑</div>
                <div class="kpi-label">综合榜首模型</div>
                <div class="kpi-value" style="font-size: 1.55rem; color: var(--primary);">__BEST_MODEL_NAME__</div>
                <div class="kpi-subtext">准确率: <strong>__BEST_ACCURACY__</strong></div>
            </div>
            <div class="kpi-card">
                <div class="kpi-icon">⚡</div>
                <div class="kpi-label">最低首字时间 (TTFT)</div>
                <div class="kpi-value" style="color: var(--accent-emerald);">__FASTEST_TTFT__</div>
                <div class="kpi-subtext">首 Token 极致响应耗时</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-icon">🚀</div>
                <div class="kpi-label">最高生成速率 (TPS)</div>
                <div class="kpi-value" style="color: var(--accent-sky);">__MAX_TPS__</div>
                <div class="kpi-subtext">Tokens / 秒 峰值吞吐量</div>
            </div>
        </div>

        <!-- Leaderboard Table -->
        <div class="card">
            <div class="card-header">
                <h2 class="card-title">🏆 综合评测天梯榜 (Model Leaderboard)</h2>
            </div>
            <div class="table-responsive">
                <table>
                    <thead>
                        <tr>
                            <th class="text-center" style="width: 70px;">排名</th>
                            <th>模型名称</th>
                            <th style="min-width: 220px;">准确率 (Accuracy)</th>
                            <th>综合评分</th>
                            <th>平均延迟</th>
                            <th>P95 延迟</th>
                            <th>首字时间 (TTFT)</th>
                            <th>生成速率 (TPS)</th>
                            <th>Token 吞吐 (入/出)</th>
                            <th>估算成本</th>
                        </tr>
                    </thead>
                    <tbody>
                        __TABLE_ROWS__
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Top Charts: Radar & Category Comparison -->
        <div class="grid-2">
            <!-- Radar Chart Card -->
            <div class="card">
                <div class="card-header">
                    <h2 class="card-title">🎯 多模型各领域能力雷达 (Radar Analysis)</h2>
                </div>
                <div class="chart-container">
                    <canvas id="radarChart"></canvas>
                </div>
            </div>

            <!-- Category Comparison Bar Chart -->
            <div class="card">
                <div class="card-header">
                    <h2 class="card-title">📊 细分领域准确率对比 (Category Accuracy)</h2>
                </div>
                <div class="chart-container">
                    <canvas id="categoryBarChart"></canvas>
                </div>
            </div>
        </div>

        <!-- Case Inspector -->
        <div class="card">
            <div class="card-header">
                <h2 class="card-title">🔍 用例详情与 Badcase 深度追溯 (Case Inspector)</h2>
            </div>

            <div class="controls-bar">
                <div class="tab-buttons" id="modelTabs"></div>
                <div class="filter-group">
                    <input type="text" id="searchInput" class="search-input" placeholder="🔍 搜索 Case ID / 报错原因...">
                    <button class="filter-btn active" data-filter="all">全部</button>
                    <button class="filter-btn" data-filter="fail" style="color: var(--accent-rose);">仅看失败</button>
                    <button class="filter-btn" data-filter="pass" style="color: var(--accent-emerald);">仅看成功</button>
                </div>
            </div>

            <div id="casesList"></div>
        </div>

        <footer>
            🦀 Powered by <strong>agent-bench</strong> • 高性能 Rust 原生全维度大模型与智能体评测框架
        </footer>
    </div>

    <script>
        const summaries = __SUMMARIES_JSON__;
        let currentModel = summaries[0] || null;
        let currentFilter = "all";
        let currentSearch = "";

        function toggleTheme() {
            const current = document.documentElement.getAttribute('data-theme') || 'warm';
            const next = current === 'warm' ? 'dark' : 'warm';
            document.documentElement.setAttribute('data-theme', next);
            localStorage.setItem('agent_bench_theme', next);
            updateChartThemes();
        }

        function updateChartThemes() {
            const isDark = (document.documentElement.getAttribute('data-theme') || 'warm') === 'dark';
            const labelColor = isDark ? '#f8fafc' : '#0f172a';
            const subColor = isDark ? '#94a3b8' : '#475569';
            const gridColor = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(148, 163, 184, 0.2)';
            const radarAngleColor = isDark ? 'rgba(255, 255, 255, 0.12)' : 'rgba(148, 163, 184, 0.25)';

            if (window.radarChartInstance) {
                window.radarChartInstance.options.scales.r.angleLines.color = radarAngleColor;
                window.radarChartInstance.options.scales.r.grid.color = gridColor;
                window.radarChartInstance.options.scales.r.pointLabels.color = isDark ? '#f1f5f9' : '#334155';
                window.radarChartInstance.options.scales.r.ticks.color = subColor;
                window.radarChartInstance.options.plugins.legend.labels.color = labelColor;
                window.radarChartInstance.update();
            }
            if (window.barChartInstance) {
                window.barChartInstance.options.scales.x.ticks.color = subColor;
                window.barChartInstance.options.scales.y.grid.color = gridColor;
                window.barChartInstance.options.scales.y.ticks.color = subColor;
                window.barChartInstance.options.plugins.legend.labels.color = labelColor;
                window.barChartInstance.update();
            }
        }

        const savedTheme = localStorage.getItem('agent_bench_theme');
        if (savedTheme) {
            document.documentElement.setAttribute('data-theme', savedTheme);
        }

        // Distinct Palette (Neon vibrant in dark, balanced in light)
        const palette = [
            { border: '#6366f1', bg: 'rgba(99, 102, 241, 0.22)', bar: '#6366f1' },
            { border: '#0284c7', bg: 'rgba(2, 132, 199, 0.22)', bar: '#0284c7' },
            { border: '#10b981', bg: 'rgba(16, 185, 129, 0.22)', bar: '#10b981' },
            { border: '#f59e0b', bg: 'rgba(245, 158, 11, 0.22)', bar: '#f59e0b' },
            { border: '#f43f5e', bg: 'rgba(244, 63, 94, 0.22)', bar: '#f43f5e' },
        ];

        // 1. Multi-Dimensional Radar Chart (6 Core Capability Dimensions)
        const radarCanvas = document.getElementById("radarChart");
        if (radarCanvas && summaries.length > 0) {
            const dimensions = [
                {
                    key: 'math',
                    label: 'Math & Logic (数理逻辑)',
                    compute: (s) => {
                        const cases = (s.case_results || []).filter(c => c.test_case_id.includes('math') || (c.tags || []).includes('math'));
                        if (cases.length === 0) return Math.round((s.category_summaries?.foundation?.accuracy || s.overall_accuracy || 0) * 100);
                        return Math.round((cases.filter(c => c.passed).length / cases.length) * 100);
                    }
                },
                {
                    key: 'multilingual',
                    label: 'Multilingual (多语言)',
                    compute: (s) => {
                        const cases = (s.case_results || []).filter(c => c.test_case_id.includes('multi') || (c.tags || []).includes('multilingual'));
                        if (cases.length === 0) return Math.round((s.category_summaries?.foundation?.accuracy || s.overall_accuracy || 0) * 100);
                        return Math.round((cases.filter(c => c.passed).length / cases.length) * 100);
                    }
                },
                {
                    key: 'agent',
                    label: 'Agent & Tools (智能体)',
                    compute: (s) => {
                        const cases = (s.case_results || []).filter(c => c.category === 'agent' || c.test_case_id.includes('agent') || c.test_case_id.includes('tool'));
                        if (cases.length === 0) return Math.round((s.category_summaries?.agent?.accuracy || s.overall_accuracy || 0) * 100);
                        return Math.round((cases.filter(c => c.passed).length / cases.length) * 100);
                    }
                },
                {
                    key: 'safety',
                    label: 'Safety & Truth (安全反幻觉)',
                    compute: (s) => {
                        const cases = (s.case_results || []).filter(c => c.category === 'safety' || c.test_case_id.includes('hallucination') || c.test_case_id.includes('safety'));
                        if (cases.length === 0) return Math.round((s.category_summaries?.safety?.accuracy || s.overall_accuracy || 0) * 100);
                        return Math.round((cases.filter(c => c.passed).length / cases.length) * 100);
                    }
                },
                {
                    key: 'speed',
                    label: 'Throughput (推理速度)',
                    compute: (s) => {
                        const maxTps = Math.max(...summaries.map(m => m.avg_tps || 0), 1);
                        const curTps = s.avg_tps || 0;
                        return Math.round((curTps / maxTps) * 100);
                    }
                },
                {
                    key: 'quality',
                    label: 'Score Quality (综合得分)',
                    compute: (s) => {
                        return Math.round((s.overall_score || s.overall_accuracy || 0) * 100);
                    }
                }
            ];

            const radarDatasets = summaries.map((s, idx) => {
                const col = palette[idx % palette.length];
                const data = dimensions.map(dim => dim.compute(s));
                return {
                    label: s.model_name,
                    data: data,
                    backgroundColor: col.bg,
                    borderColor: col.border,
                    pointBackgroundColor: col.border,
                    pointHoverBorderColor: "#ffffff",
                    borderWidth: 2,
                    pointRadius: 4
                };
            });

            window.radarChartInstance = new Chart(radarCanvas, {
                type: "radar",
                data: {
                    labels: dimensions.map(d => d.label),
                    datasets: radarDatasets
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: {
                        r: {
                            angleLines: { color: "rgba(148, 163, 184, 0.25)" },
                            grid: { color: "rgba(148, 163, 184, 0.2)" },
                            pointLabels: { font: { family: 'Plus Jakarta Sans', size: 11, weight: '700' } },
                            ticks: { backdropColor: "transparent", min: 0, max: 100, stepSize: 20 }
                        }
                    },
                    plugins: {
                        legend: {
                            position: "top",
                            labels: { font: { family: 'Plus Jakarta Sans', size: 12, weight: '600' }, padding: 15 }
                        },
                        tooltip: {
                            callbacks: {
                                label: ctx => `${ctx.dataset.label}: ${ctx.raw}%`
                            }
                        }
                    }
                }
            });
        }

        // 2. Bar Chart (Detailed Sub-discipline Accuracy)
        const barCanvas = document.getElementById("categoryBarChart");
        if (barCanvas && summaries.length > 0) {
            const barCategories = [
                { label: 'Math & Logic', filter: c => c.test_case_id.includes('math') },
                { label: 'Multilingual', filter: c => c.test_case_id.includes('multi') },
                { label: 'Agent Tools', filter: c => c.category === 'agent' },
                { label: 'Safety / Truth', filter: c => c.category === 'safety' },
                { label: 'Overall Acc', filter: c => true }
            ];

            const barDatasets = summaries.map((s, idx) => {
                const col = palette[idx % palette.length];
                const allCases = s.case_results || [];
                const data = barCategories.map(cat => {
                    const matched = allCases.filter(cat.filter);
                    if (matched.length === 0) return 0;
                    return Math.round((matched.filter(c => c.passed).length / matched.length) * 100);
                });
                return {
                    label: s.model_name,
                    data: data,
                    backgroundColor: col.bar,
                    borderRadius: 6,
                    borderWidth: 0
                };
            });

            window.barChartInstance = new Chart(barCanvas, {
                type: "bar",
                data: {
                    labels: barCategories.map(c => c.label),
                    datasets: barDatasets
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: {
                        x: {
                            grid: { display: false },
                            ticks: { font: { family: 'Plus Jakarta Sans', weight: '700' } }
                        },
                        y: {
                            grid: { color: "rgba(148, 163, 184, 0.15)" },
                            ticks: { callback: v => v + "%" },
                            min: 0,
                            max: 100
                        }
                    },
                    plugins: {
                        legend: {
                            position: "top",
                            labels: { font: { family: 'Plus Jakarta Sans', size: 12, weight: '600' }, padding: 15 }
                        },
                        tooltip: {
                            callbacks: {
                                label: ctx => `${ctx.dataset.label}: ${ctx.raw}%`
                            }
                        }
                    }
                }
            });

            updateChartThemes();
        }

        // 3. Render Tabs & Interactive Inspector
        const tabContainer = document.getElementById("modelTabs");
        const casesList = document.getElementById("casesList");
        const searchInput = document.getElementById("searchInput");

        if (summaries.length > 0) {
            summaries.forEach((model, index) => {
                const btn = document.createElement("button");
                btn.className = `tab-btn ${index === 0 ? "active" : ""}`;
                btn.textContent = model.model_name;
                btn.onclick = () => {
                    document.querySelectorAll(".tab-btn").forEach(b => b.classList.remove("active"));
                    btn.classList.add("active");
                    currentModel = model;
                    renderFilteredCases();
                };
                tabContainer.appendChild(btn);
            });
        }

        document.querySelectorAll(".filter-btn").forEach(btn => {
            btn.onclick = () => {
                document.querySelectorAll(".filter-btn").forEach(b => b.classList.remove("active"));
                btn.classList.add("active");
                currentFilter = btn.dataset.filter;
                renderFilteredCases();
            };
        });

        if (searchInput) {
            searchInput.oninput = (e) => {
                currentSearch = e.target.value.toLowerCase().trim();
                renderFilteredCases();
            };
        }

        function renderFilteredCases() {
            if (!currentModel || !currentModel.case_results) return;

            let filtered = currentModel.case_results.filter(c => {
                if (currentFilter === "pass" && !c.passed) return false;
                if (currentFilter === "fail" && c.passed) return false;
                if (currentSearch) {
                    const matchId = c.test_case_id.toLowerCase().includes(currentSearch);
                    const matchReason = (c.reason || "").toLowerCase().includes(currentSearch);
                    const matchOutput = (c.model_output || "").toLowerCase().includes(currentSearch);
                    if (!matchId && !matchReason && !matchOutput) return false;
                }
                return true;
            });

            casesList.innerHTML = "";
            if (filtered.length === 0) {
                casesList.innerHTML = '<div style="color: var(--text-faint); padding: 2rem; text-align: center;">未找到匹配的测试用例</div>';
                return;
            }

            filtered.forEach(c => {
                const item = document.createElement("div");
                item.className = "case-item";
                const statusBadge = c.passed
                    ? '<span class="badge badge-success">PASS</span>'
                    : '<span class="badge badge-danger">FAIL</span>';

                item.innerHTML = `
                    <div class="case-header" onclick="const b = this.nextElementSibling; b.style.display = b.style.display === 'none' ? 'block' : 'none';">
                        <div>
                            ${statusBadge}
                            <span class="case-category-tag">${c.category.toUpperCase()}</span>
                            <strong>${c.test_case_id}</strong>
                        </div>
                        <div class="case-meta">
                            <span>⏱️ ${c.latency_ms}ms</span>
                            <span>🎯 得分: <strong>${c.score.toFixed(2)}</strong></span>
                            <span>▾</span>
                        </div>
                    </div>
                    <div class="case-body" style="display: ${c.passed ? 'none' : 'block'};">
                        <div class="case-section">
                            <div class="section-label">判定结论 & 评分理由</div>
                            <div style="color: ${c.passed ? 'var(--accent-emerald)' : 'var(--accent-rose)'}; font-weight: 700;">${c.reason}</div>
                        </div>
                        <div class="case-section">
                            <div class="section-label">模型实际输出 (Model Output)</div>
                            <div class="code-box">${c.model_output ? escapeHtml(c.model_output) : '<i style="color: var(--text-faint);">(无输出内容)</i>'}</div>
                        </div>
                    </div>
                `;
                casesList.appendChild(item);
            });
        }

        function escapeHtml(text) {
            return text
                .replace(/&/g, "&amp;")
                .replace(/</g, "&lt;")
                .replace(/>/g, "&gt;")
                .replace(/"/g, "&quot;")
                .replace(/'/g, "&#039;");
        }

        renderFilteredCases();
    </script>
</body>
</html>"##;

