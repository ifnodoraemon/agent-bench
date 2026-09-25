/**
 * Export tools for Vue 3 benchmark dashboard
 */

export function exportLeaderboardCsv(summaries) {
  if (!summaries || summaries.length === 0) return;

  const headers = [
    "Rank", "Model Name", "Model ID", "Elo Rating",
    "Micro Accuracy (%)", "Macro Accuracy (%)", "L4/L5 Frontier (%)",
    "Composite Score", "Avg Latency (ms)", "P95 Latency (ms)",
    "TTFT (ms)", "TPS (tok/s)", "Tokens Prompt", "Tokens Completion", "Cost ($ USD)"
  ];

  const rows = summaries.map((s, idx) => [
    idx + 1,
    `"${s.model_name || s.model_id}"`,
    `"${s.model_id}"`,
    Math.round(s.computed_elo || s.elo_rating || 1200),
    ((s.overall_accuracy || 0) * 100).toFixed(2),
    ((s.macro_accuracy || 0) * 100).toFixed(2),
    ((s.l4_l5_frontier_accuracy || 0) * 100).toFixed(2),
    (s.weighted_composite_index || s.overall_score * 100 || 0).toFixed(1),
    Math.round(s.avg_latency_ms || 0),
    Math.round(s.p95_latency_ms || 0),
    s.avg_ttft_ms ? Math.round(s.avg_ttft_ms) : "-",
    (s.avg_tps || 0).toFixed(1),
    s.total_prompt_tokens || 0,
    s.total_completion_tokens || 0,
    (s.total_cost_usd || 0).toFixed(4)
  ]);

  downloadCsv([headers, ...rows], "agent_bench_leaderboard.csv");
}

export function exportCasesCsv(cases, filename = "badcases.csv") {
  if (!cases || cases.length === 0) return;

  const headers = [
    "Test Case ID", "Category", "Difficulty", "Passed",
    "Score", "Latency (ms)", "Tokens Prompt", "Tokens Completion",
    "Evaluation Reason", "Model Output", "Reasoning Content"
  ];

  const rows = cases.map(c => [
    `"${c.test_case_id}"`,
    `"${c.category}"`,
    `"${c.resolved_tier || c.difficulty || ''}"`,
    c.passed ? "PASS" : "FAIL",
    (c.score || 0).toFixed(4),
    Math.round(c.latency_ms || 0),
    c.prompt_tokens || 0,
    c.completion_tokens || 0,
    `"${(c.reason || '').replace(/"/g, '""')}"`,
    `"${(c.model_output || '').replace(/"/g, '""')}"`,
    `"${(c.reasoning_content || '').replace(/"/g, '""')}"`
  ]);

  downloadCsv([headers, ...rows], filename);
}

export function exportModelCasesCsv(model) {
  if (!model || !model.case_results) return;
  exportCasesCsv(model.case_results, `eval_${model.model_id}_cases.csv`);
}

function downloadCsv(data, filename) {
  const csvContent = "data:text/csv;charset=utf-8,\uFEFF" + data.map(r => r.join(",")).join("\n");
  const encodedUri = encodeURI(csvContent);
  const link = document.createElement("a");
  link.setAttribute("href", encodedUri);
  link.setAttribute("download", filename);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}
