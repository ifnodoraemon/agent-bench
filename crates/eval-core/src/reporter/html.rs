use crate::metrics::ModelBenchmarkSummary;
use chrono::Local;

pub struct HtmlReporter;

impl HtmlReporter {
    pub fn generate_html(summaries: &[ModelBenchmarkSummary]) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let summaries_json = serde_json::to_string(summaries).unwrap_or_else(|_| "[]".to_string());

        let table_rows = summaries
            .iter()
            .map(|s| {
                let ttft = s
                    .avg_ttft_ms
                    .map(|t| format!("{:.0}ms", t))
                    .unwrap_or_else(|| "-".to_string());
                let acc_class = if s.overall_accuracy >= 0.8 {
                    "badge-success"
                } else if s.overall_accuracy >= 0.5 {
                    "badge-warning"
                } else {
                    "badge-danger"
                };

                format!(
                    r#"<tr>
                    <td><strong>{}</strong></td>
                    <td><span class="badge {}">{:.1}% ({}/{})</span></td>
                    <td><strong>{:.2}</strong></td>
                    <td>{:.0}ms</td>
                    <td>{:.0}ms</td>
                    <td>{}</td>
                    <td>{:.1}</td>
                    <td>{}/{}</td>
                    <td>${:.5}</td>
                </tr>"#,
                    s.model_name,
                    acc_class,
                    s.overall_accuracy * 100.0,
                    s.passed_cases,
                    s.total_cases,
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

        format!(
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🦀 agent-bench 全维度大模型与 Agent 评测大屏</title>
    <!-- Chart.js CDN -->
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        :root {{
            --bg: #0b0f19;
            --surface: #151c2e;
            --surface-hover: #1e293b;
            --border: #2d3748;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --accent-blue: #38bdf8;
            --accent-purple: #a855f7;
            --accent-emerald: #10b981;
            --accent-rose: #f43f5e;
            --accent-amber: #f59e0b;
        }}
        * {{ box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background-color: var(--bg);
            color: var(--text-primary);
            margin: 0;
            padding: 2rem;
            line-height: 1.5;
        }}
        .container {{
            max-width: 1300px;
            margin: 0 auto;
        }}
        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border);
            padding-bottom: 1.5rem;
            margin-bottom: 2rem;
        }}
        h1 {{
            margin: 0;
            font-size: 1.8rem;
            background: linear-gradient(135deg, var(--accent-blue), var(--accent-purple));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .timestamp {{
            color: var(--text-secondary);
            font-size: 0.9rem;
        }}
        .grid-2 {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(550px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }}
        .card {{
            background: var(--surface);
            border-radius: 12px;
            padding: 1.5rem;
            border: 1px solid var(--border);
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
        }}
        .card-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 1rem;
        }}
        .card-title {{
            font-size: 1.15rem;
            font-weight: 600;
            color: var(--text-primary);
            margin: 0;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .chart-container {{
            position: relative;
            height: 340px;
            width: 100%;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            text-align: left;
            font-size: 0.9rem;
        }}
        th, td {{
            padding: 0.85rem 1rem;
            border-bottom: 1px solid var(--border);
        }}
        th {{
            background-color: rgba(255, 255, 255, 0.03);
            color: var(--text-secondary);
            font-weight: 600;
            text-transform: uppercase;
            font-size: 0.8rem;
            letter-spacing: 0.05em;
        }}
        tr:hover {{
            background-color: var(--surface-hover);
        }}
        .badge {{
            display: inline-block;
            padding: 0.2rem 0.55rem;
            border-radius: 9999px;
            font-weight: 600;
            font-size: 0.8rem;
        }}
        .badge-success {{ background: rgba(16, 185, 129, 0.15); color: var(--accent-emerald); border: 1px solid rgba(16, 185, 129, 0.3); }}
        .badge-warning {{ background: rgba(245, 158, 11, 0.15); color: var(--accent-amber); border: 1px solid rgba(245, 158, 11, 0.3); }}
        .badge-danger {{ background: rgba(244, 63, 94, 0.15); color: var(--accent-rose); border: 1px solid rgba(244, 63, 94, 0.3); }}
        
        .tab-buttons {{
            display: flex;
            gap: 0.5rem;
            margin-bottom: 1rem;
        }}
        .tab-btn {{
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid var(--border);
            color: var(--text-secondary);
            padding: 0.4rem 1rem;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.85rem;
            transition: all 0.2s;
        }}
        .tab-btn.active {{
            background: var(--accent-blue);
            color: #0b0f19;
            font-weight: bold;
            border-color: var(--accent-blue);
        }}
        .case-item {{
            background: rgba(0, 0, 0, 0.2);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1rem;
            margin-bottom: 0.75rem;
        }}
        .case-header {{
            display: flex;
            justify-content: space-between;
            cursor: pointer;
            font-weight: 600;
        }}
        .case-body {{
            margin-top: 0.75rem;
            padding-top: 0.75rem;
            border-top: 1px dashed var(--border);
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
            font-size: 0.85rem;
            color: var(--text-secondary);
            white-space: pre-wrap;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div>
                <h1>🦀 agent-bench 综合能力评测大屏</h1>
                <div class="timestamp">全维度模型对比 • 基础推理 • Agent 智能体 • 安全防御 • 性价比</div>
            </div>
            <div class="timestamp">报告生成时间: {}</div>
        </header>

        <!-- Top Charts: Radar & Scatter Plot -->
        <div class="grid-2">
            <!-- Radar Chart Card -->
            <div class="card">
                <div class="card-header">
                    <h2 class="card-title">🎯 多模型全维度能力雷达图 (Radar Chart)</h2>
                </div>
                <div class="chart-container">
                    <canvas id="radarChart"></canvas>
                </div>
            </div>

            <!-- Scatter Plot Card -->
            <div class="card">
                <div class="card-header">
                    <h2 class="card-title">⚡ 性价比象限图 (Accuracy vs Latency)</h2>
                </div>
                <div class="chart-container">
                    <canvas id="scatterChart"></canvas>
                </div>
            </div>
        </div>

        <!-- Leaderboard Table -->
        <div class="card" style="margin-bottom: 2rem;">
            <div class="card-header">
                <h2 class="card-title">🏆 综合评测天梯榜 (Leaderboard)</h2>
            </div>
            <table>
                <thead>
                    <tr>
                        <th>模型名称</th>
                        <th>准确率 (Accuracy)</th>
                        <th>综合得分</th>
                        <th>平均延迟</th>
                        <th>P95 延迟</th>
                        <th>首字延迟 (TTFT)</th>
                        <th>生成速率 (TPS)</th>
                        <th>Token 吞吐 (入/出)</th>
                        <th>总成本 ($)</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>

        <!-- Case Inspector -->
        <div class="card">
            <div class="card-header">
                <h2 class="card-title">🔍 用例详情与 Badcase 分析</h2>
            </div>
            <div class="tab-buttons" id="modelTabs"></div>
            <div id="casesList"></div>
        </div>
    </div>

    <script>
        const summaries = {};

        // 1. Initialize Radar Chart
        const radarCanvas = document.getElementById('radarChart');
        if (radarCanvas && summaries.length > 0) {{
            // Collect all unique categories
            const categorySet = new Set();
            summaries.forEach(s => {{
                Object.keys(s.category_summaries || {{}}).forEach(cat => categorySet.add(cat));
            }});
            const categories = Array.from(categorySet);
            if (categories.length === 0) categories.push('foundation', 'agent', 'safety');

            const colors = [
                {{ border: '#38bdf8', bg: 'rgba(56, 189, 248, 0.2)' }},
                {{ border: '#a855f7', bg: 'rgba(168, 85, 247, 0.2)' }},
                {{ border: '#10b981', bg: 'rgba(16, 185, 129, 0.2)' }},
                {{ border: '#f43f5e', bg: 'rgba(244, 63, 94, 0.2)' }},
                {{ border: '#f59e0b', bg: 'rgba(245, 158, 11, 0.2)' }},
            ];

            const radarDatasets = summaries.map((s, idx) => {{
                const color = colors[idx % colors.length];
                const data = categories.map(cat => {{
                    const catStat = s.category_summaries && s.category_summaries[cat];
                    return catStat ? Math.round(catStat.accuracy * 100) : 0;
                }});
                return {{
                    label: s.model_name,
                    data: data,
                    backgroundColor: color.bg,
                    borderColor: color.border,
                    pointBackgroundColor: color.border,
                    pointHoverBorderColor: '#fff',
                    borderWidth: 2
                }};
            }});

            new Chart(radarCanvas, {{
                type: 'radar',
                data: {{
                    labels: categories.map(c => c.toUpperCase()),
                    datasets: radarDatasets
                }},
                options: {{
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: {{
                        r: {{
                            angleLines: {{ color: 'rgba(255, 255, 255, 0.1)' }},
                            grid: {{ color: 'rgba(255, 255, 255, 0.08)' }},
                            pointLabels: {{ color: '#94a3b8', font: {{ size: 12, weight: 'bold' }} }},
                            ticks: {{ color: '#64748b', backdropColor: 'transparent', min: 0, max: 100 }}
                        }}
                    }},
                    plugins: {{
                        legend: {{ labels: {{ color: '#f8fafc' }} }}
                    }}
                }}
            }});
        }}

        // 2. Initialize Scatter Plot (Latency vs Accuracy)
        const scatterCanvas = document.getElementById('scatterChart');
        if (scatterCanvas && summaries.length > 0) {{
            const colors = ['#38bdf8', '#a855f7', '#10b981', '#f43f5e', '#f59e0b'];
            const scatterDatasets = summaries.map((s, idx) => ({{
                label: s.model_name,
                data: [{{
                    x: Math.round(s.avg_latency_ms),
                    y: Math.round(s.overall_accuracy * 100),
                    r: Math.max(8, Math.min(20, s.avg_tps / 5))
                }}],
                backgroundColor: colors[idx % colors.length],
                borderColor: '#ffffff',
                borderWidth: 1.5
            }}));

            new Chart(scatterCanvas, {{
                type: 'bubble',
                data: {{ datasets: scatterDatasets }},
                options: {{
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: {{
                        x: {{
                            title: {{ display: true, text: '平均延迟 (Latency ms) - 越低越好', color: '#94a3b8' }},
                            grid: {{ color: 'rgba(255, 255, 255, 0.08)' }},
                            ticks: {{ color: '#64748b' }}
                        }},
                        y: {{
                            title: {{ display: true, text: '综合准确率 (Accuracy %) - 越高越好', color: '#94a3b8' }},
                            grid: {{ color: 'rgba(255, 255, 255, 0.08)' }},
                            ticks: {{ color: '#64748b', min: 0, max: 100 }}
                        }}
                    }},
                    plugins: {{
                        legend: {{ labels: {{ color: '#f8fafc' }} }},
                        tooltip: {{
                            callbacks: {{
                                label: function(ctx) {{
                                    return `${{ctx.dataset.label}}: 准确率 ${{ctx.raw.y}}%, 延迟 ${{ctx.raw.x}}ms`;
                                }}
                            }}
                        }}
                    }}
                }}
            }});
        }}

        // 3. Render Tabs & Cases
        const tabContainer = document.getElementById('modelTabs');
        const casesList = document.getElementById('casesList');

        if (summaries.length > 0) {{
            summaries.forEach((model, index) => {{
                const btn = document.createElement('button');
                btn.className = `tab-btn ${{index === 0 ? 'active' : ''}}`;
                btn.textContent = model.model_name;
                btn.onclick = () => {{
                    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
                    btn.classList.add('active');
                    renderCases(model);
                }};
                tabContainer.appendChild(btn);
            }});
            renderCases(summaries[0]);
        }}

        function renderCases(model) {{
            casesList.innerHTML = '';
            if (!model.case_results || model.case_results.length === 0) {{
                casesList.innerHTML = '<div style="color: var(--text-secondary);">暂无测试用例数据</div>';
                return;
            }}

            model.case_results.forEach((c, i) => {{
                const item = document.createElement('div');
                item.className = 'case-item';
                const statusBadge = c.passed ? '<span class="badge badge-success">PASS</span>' : '<span class="badge badge-danger">FAIL</span>';
                
                item.innerHTML = `
                    <div class="case-header" onclick="this.nextElementSibling.style.display = this.nextElementSibling.style.display === 'none' ? 'block' : 'none'">
                        <span>${{statusBadge}} <strong style="margin-left: 0.5rem;">[${{c.category}}]</strong> ${{c.test_case_id}}</span>
                        <span style="color: var(--text-secondary); font-size: 0.85rem;">${{c.latency_ms}}ms | 得分: ${{c.score.toFixed(2)}} ▾</span>
                    </div>
                    <div class="case-body" style="display: ${{c.passed ? 'none' : 'block'}};">
<strong>判定结论:</strong> ${{c.reason}}
<strong>模型输出:</strong>
${{c.model_output || '(无输出内容)'}}
                    </div>
                `;
                casesList.appendChild(item);
            }});
        }}
    </script>
</body>
</html>"#,
            now,
            table_rows,
            summaries_json
        )
    }
}
