/**
 * Benchmark data processing and categorization utilities
 */

export const DOMAIN_DEFINITIONS = [
  {
    key: 'swe',
    name: '软件工程 (SWE)',
    icon: '💻',
    categories: ['swe', 'code_generation', 'coding'],
    description: '真实代码修复、功能演进、单元测试与重构'
  },
  {
    key: 'agent',
    name: '智能体运维 (Agent & Ops)',
    icon: '🤖',
    categories: ['agent', 'devops', 'security', 'data_analyst'],
    description: '自主工具调用、多步ReAct、DevOps部署与安全对抗'
  },
  {
    key: 'science',
    name: '自然科学医疗 (Science & Med)',
    icon: '🔬',
    categories: ['science', 'medical'],
    description: '生命医学、物理化学、临床推论与论文推理'
  },
  {
    key: 'law_finance',
    name: '法律金融 (Law & Finance)',
    icon: '⚖️',
    categories: ['legal', 'finance'],
    description: '法条检索适用、财务报表分析与量化建模'
  },
  {
    key: 'math_logic',
    name: '数理逻辑 (Math & Logic)',
    icon: '📐',
    categories: ['math_logic', 'knowledge_math'],
    description: '复杂数论、离散数学、Olympiad竞赛与逻辑推理'
  },
  {
    key: 'humanities',
    name: '人文多语言 (Humanities & Lang)',
    icon: '📚',
    categories: ['humanities', 'multilingual'],
    description: '跨语言翻译、历史哲学辩论与深度内容创编'
  },
  {
    key: 'safety',
    name: '安全与对齐 (Safety & Align)',
    icon: '🛡️',
    categories: ['safety', 'instruction', 'structured_output', 'long_context'],
    description: '越狱注入防御、隐私脱敏、严格格式遵守与长文本针检索'
  }
];

export const CATEGORY_NAMES = {
  swe: '软件工程 (SWE)',
  code_generation: '代码生成 (CodeGen)',
  agent: '多轮Agent (Agent)',
  devops: '运维管理 (DevOps)',
  security: '网络安全 (Security)',
  data_analyst: '数据分析 (Data Analyst)',
  science: '自然科学 (Science)',
  medical: '临床医学 (Medical)',
  legal: '法律合规 (Legal)',
  finance: '金融经济 (Finance)',
  math_logic: '数理逻辑 (Math & Logic)',
  knowledge_math: '综合数学 (Knowledge Math)',
  humanities: '人文哲学 (Humanities)',
  multilingual: '多语言翻译 (Multilingual)',
  safety: '安全与对齐 (Safety)',
  instruction: '指令遵循 (IFEval)',
  structured_output: '结构化JSON (Structured Output)',
  long_context: '长文本 Needle (Long Context)'
};

export const TIER_DEFINITIONS = {
  L1: { name: 'L1 基础自洽 (Sanity)', color: '#0ea5e9', desc: '基础指令遵循、JSON格式生成、敏感词脱敏' },
  L2: { name: 'L2 标准任务 (Standard)', color: '#10b981', desc: '单文件代码生成、跨语言翻译、长文本针检索' },
  L3: { name: 'L3 专业领域 (Specialist)', color: '#f59e0b', desc: '医考病案、司法案例裁决、公司财报分析、高难度数学' },
  L4: { name: 'L4 自主智能体 (Autonomous Agent)', color: '#a855f7', desc: '多步工具调用、Bash命令执行、复杂环境排障重试' },
  L5: { name: 'L5 极限难题 (Frontier & Olympiad)', color: '#ef4444', desc: 'SWE-bench级跨文件Bug定位、复杂越狱对抗防御' }
};

const DIFFICULTY_RULES = [
  { tier: 'L5', matchAny: ['_hard_', 'putnam', 'swe_hard', 'jailbreak'] },
  { tier: 'L4', prefixAny: ['agent', 'sec', 'devops', 'swe', 'react', 'tool', 'error'] },
  { tier: 'L3', prefixAny: ['med', 'law', 'legal', 'fin', 'math'], categories: ['medical', 'legal', 'finance', 'math_logic'] },
  { tier: 'L2', prefixAny: ['hum', 'sci', 'multi', 'needle', 'code', 'hallucination'] },
];

/**
 * Resolve difficulty tier (L1 to L5) using declarative pattern matching table
 */
export function resolveDifficulty(c) {
  const diff = (c.difficulty || '').trim().toUpperCase();
  if (TIER_DEFINITIONS[diff]) return diff;

  const id = (c.test_case_id || c.id || '').toLowerCase();
  const cat = c.category;

  const matched = DIFFICULTY_RULES.find(rule => {
    if (rule.matchAny && rule.matchAny.some(k => id.includes(k))) return true;
    if (rule.prefixAny && rule.prefixAny.some(k => id.startsWith(k))) return true;
    if (rule.categories && rule.categories.includes(cat)) return true;
    return false;
  });

  return matched ? matched.tier : 'L1';
}

/**
 * Extract thinking trace from output
 */
export function extractThinking(output) {
  if (!output || typeof output !== 'string') {
    return { thinking: '', response: '' };
  }

  const thinkMatch = output.match(/<think(?:ing)?>([\s\S]*?)<\/think(?:ing)?>/i);
  if (thinkMatch) {
    const thinking = thinkMatch[1].trim();
    const response = output.replace(/<think(?:ing)?>[\s\S]*?<\/think(?:ing)?>/gi, '').trim();
    return { thinking, response };
  }

  const thoughtMatch = output.match(/<thought>([\s\S]*?)<\/thought>/i);
  if (thoughtMatch) {
    const thinking = thoughtMatch[1].trim();
    const response = output.replace(/<thought>[\s\S]*?<\/thought>/gi, '').trim();
    return { thinking, response };
  }

  return { thinking: '', response: output };
}

/**
 * Enrich model summary with computed tier statistics, domain scores, and frontier accuracy
 */
export function enrichModelSummary(summary) {
  const cases = summary.case_results || [];

  // Tiers calculation
  const tiers = {
    L1: { total: 0, passed: 0, accuracy: 0, avg_latency: 0, avg_tps: 0 },
    L2: { total: 0, passed: 0, accuracy: 0, avg_latency: 0, avg_tps: 0 },
    L3: { total: 0, passed: 0, accuracy: 0, avg_latency: 0, avg_tps: 0 },
    L4: { total: 0, passed: 0, accuracy: 0, avg_latency: 0, avg_tps: 0 },
    L5: { total: 0, passed: 0, accuracy: 0, avg_latency: 0, avg_tps: 0 }
  };

  const domainTotals = {};
  DOMAIN_DEFINITIONS.forEach(d => {
    domainTotals[d.key] = { total: 0, passed: 0, score: 0 };
  });

  let l4l5Total = 0;
  let l4l5Passed = 0;

  cases.forEach(c => {
    const tier = resolveDifficulty(c);
    c.resolved_tier = tier; // Cache on case
    if (tiers[tier]) {
      tiers[tier].total++;
      if (c.passed) tiers[tier].passed++;
    }

    if (tier === 'L4' || tier === 'L5') {
      l4l5Total++;
      if (c.passed) l4l5Passed++;
    }

    // Domain tally
    for (const d of DOMAIN_DEFINITIONS) {
      if (d.categories.includes(c.category)) {
        domainTotals[d.key].total++;
        if (c.passed) domainTotals[d.key].passed++;
        domainTotals[d.key].score += (c.score || 0);
        break;
      }
    }
  });

  // Calculate percentages
  Object.keys(tiers).forEach(k => {
    const t = tiers[k];
    t.accuracy = t.total > 0 ? (t.passed / t.total) : 0;
  });

  const domainScores = {};
  DOMAIN_DEFINITIONS.forEach(d => {
    const dt = domainTotals[d.key];
    domainScores[d.key] = dt.total > 0 ? (dt.passed / dt.total) : 0;
  });

  const frontierAccuracy = l4l5Total > 0 ? (l4l5Passed / l4l5Total) : 0;

  const { canonical, channel } = resolveCanonicalModelAndChannel(
    summary.model_id,
    summary.model_name,
    summary.provider
  );

  return {
    ...summary,
    canonical_model: summary.canonical_name || canonical,
    channel: summary.channel || channel,
    tier_breakdown: tiers,
    l4_l5_frontier_accuracy: frontierAccuracy,
    domain_scores: domainScores
  };
}

export const CHANNEL_RULES = [
  { channel: '硅基流动 (SiliconFlow)', match: ['silicon'] },
  { channel: '火山方舟 (Volcengine)', match: ['volc', 'ark', 'ep-'] },
  { channel: 'OpenRouter', match: ['openrouter'] },
  { channel: '阿里云百炼 (DashScope)', match: ['dashscope', 'bailian', 'aliyun'] },
  { channel: 'Together AI', match: ['together'] },
  { channel: 'Groq (LPU)', match: ['groq'] },
  { channel: 'AWS Bedrock', match: ['bedrock'] },
  { channel: 'GCP Vertex AI', match: ['vertex'] },
  { channel: 'Azure AI Foundry', match: ['azure'] },
  { channel: 'GPUStack 企业算力', match: ['gpustack', '10.232.'] },
  { channel: '私有集群 (Self-Hosted/vLLM)', match: ['vllm', 'sglang', 'local', 'ollama'] },
  { channel: '模拟沙盒 (Mock Sandbox)', match: ['mock'] },
];

export function resolveChannel(modelId, provider) {
  const combined = `${modelId || ''} ${provider || ''}`.toLowerCase();
  const rule = CHANNEL_RULES.find(r => r.match.some(k => combined.includes(k)));
  return rule ? rule.channel : '官方直连 (Official)';
}

export const CANONICAL_RULES = [
  // 2026 Flagship Frontier First
  {
    canonical_id: 'claude-opus-5-5',
    display_name: 'Claude-Opus-5.5',
    match: ['claude-opus-5.5', 'claude-opus-5-5', 'claude-5.5', 'claude-5', 'opus-5.5', 'opus-5'],
    negative: []
  },
  {
    canonical_id: 'gpt-6-astra',
    display_name: 'GPT-6-Astra',
    match: ['gpt-6-astra', 'gpt-6-sol', 'gpt-6-luna', 'gpt-6', 'gpt6'],
    negative: []
  },
  {
    canonical_id: 'kimi-k3',
    display_name: 'Kimi-K3',
    match: ['kimi-k3', 'kimi_k3', 'kimik3', 'kimi-3', 'kimi3'],
    negative: []
  },
  {
    canonical_id: 'qwen-3-8-max',
    display_name: 'Qwen-3.8-Max',
    match: ['qwen-3.8-max', 'qwen-3.8', 'qwen3.8', 'qwen-3', 'qwen3'],
    negative: ['qwq']
  },
  {
    canonical_id: 'gemini-3-8-flash',
    display_name: 'Gemini-3.8-Flash',
    match: ['gemini-3.8', 'gemini-3.1', 'gemini-3', 'gemini3'],
    negative: []
  },
  {
    canonical_id: 'llama-4-maverick',
    display_name: 'Llama-4-Maverick',
    match: ['llama-4-maverick', 'llama-4-scout', 'llama-4', 'llama4'],
    negative: []
  },
  // DeepSeek Family
  {
    canonical_id: 'deepseek-v4',
    display_name: 'DeepSeek-V4',
    match: ['deepseek-v4', 'deepseek_v4', 'deepseekv4'],
    negative: []
  },
  {
    canonical_id: 'deepseek-r1',
    display_name: 'DeepSeek-R1',
    match: ['deepseek-r1', 'deepseek_r1', 'deepseek-reasoner', 'r1-distill', 'r1'],
    negative: []
  },
  {
    canonical_id: 'deepseek-v3',
    display_name: 'DeepSeek-V3',
    match: ['deepseek-v3', 'deepseek_v3', 'deepseek-chat'],
    negative: []
  },
  // xAI Grok Family
  {
    canonical_id: 'grok-3',
    display_name: 'Grok-3',
    match: ['grok-3', 'grok3', 'grok-4', 'grok4', 'grok'],
    negative: []
  },
  // Anthropic Claude Family
  {
    canonical_id: 'claude-3-7-sonnet',
    display_name: 'Claude-3.7-Sonnet',
    match: ['claude-3-7', 'claude-3.7', 'claude-37'],
    negative: ['opus-5', 'claude-5', '5.5']
  },
  {
    canonical_id: 'claude-3-5-sonnet',
    display_name: 'Claude-3.5-Sonnet',
    match: ['claude-3-5', 'claude-3.5', 'claude-35', 'sonnet'],
    negative: ['3.7', '3-7', '37', 'opus-5', 'claude-5', '5.5']
  },
  // OpenAI Family
  {
    canonical_id: 'openai-o3-mini',
    display_name: 'OpenAI-o3-mini',
    match: ['o3-mini', 'o3mini', 'openai-o3', 'o3'],
    negative: []
  },
  {
    canonical_id: 'openai-o1',
    display_name: 'OpenAI-o1',
    match: ['o1-preview', 'o1-mini', 'openai-o1', 'o1'],
    negative: []
  },
  {
    canonical_id: 'gpt-4o-mini',
    display_name: 'GPT-4o-mini',
    match: ['gpt-4o-mini', 'gpt4o-mini'],
    negative: []
  },
  {
    canonical_id: 'gpt-4o',
    display_name: 'GPT-4o',
    match: ['gpt-4o', 'gpt4o', 'gpt-5', 'gpt5'],
    negative: ['mini', 'gpt-6', 'gpt6']
  },
  // Google Gemini Family
  {
    canonical_id: 'gemini-2-0-pro',
    display_name: 'Gemini-2.0-Pro',
    match: ['gemini-2.0-pro', 'gemini-2-pro', 'gemini-pro-2'],
    negative: ['gemini-3', 'gemini3']
  },
  {
    canonical_id: 'gemini-2-0-flash',
    display_name: 'Gemini-2.0-Flash',
    match: ['gemini-2.0-flash', 'gemini-2-flash', 'gemini-2.0', 'gemini-2'],
    negative: ['pro', '1.5', 'gemini-3', 'gemini3']
  },
  {
    canonical_id: 'gemini-1-5-pro',
    display_name: 'Gemini-1.5-Pro',
    match: ['gemini-1.5', 'gemini-1-5'],
    negative: []
  },
  // Alibaba Qwen Family
  {
    canonical_id: 'qwq-32b',
    display_name: 'QwQ-32B',
    match: ['qwq-32b', 'qwq'],
    negative: []
  },
  {
    canonical_id: 'qwen-2-5-72b',
    display_name: 'Qwen-2.5-72B',
    match: ['qwen-2.5', 'qwen2.5'],
    negative: ['qwq', 'qwen-3', 'qwen3']
  },
  // Meta Llama Family
  {
    canonical_id: 'llama-3-3-70b',
    display_name: 'Llama-3.3-70B',
    match: ['llama-3.3', 'llama-3-3', 'llama3.3'],
    negative: ['llama-4', 'llama4']
  },
  // GLM Family
  {
    canonical_id: 'glm-5',
    display_name: 'GLM-5 / GLM-4',
    match: ['glm-5', 'glm-4', 'glm'],
    negative: []
  },
  // Mock Family
  {
    canonical_id: 'mock-pro-v1',
    display_name: 'Mock-Pro-v1',
    match: ['mock-pro'],
    negative: []
  },
  {
    canonical_id: 'mock-fast-v1',
    display_name: 'Mock-Fast-v1',
    match: ['mock-fast', 'mock'],
    negative: ['pro']
  }
];

export function resolveCanonicalId(rawName) {
  if (!rawName) return null;
  const lower = rawName.toLowerCase();
  const rule = CANONICAL_RULES.find(r =>
    r.match.some(k => lower.includes(k)) &&
    !r.negative.some(k => lower.includes(k))
  );
  return rule ? rule.canonical_id : null;
}

/**
 * Resolves a model's canonical family and provider channel using declarative rule tables
 */
export function resolveCanonicalModelAndChannel(modelId, modelName, provider) {
  const combined = `${modelId || ''} ${modelName || ''}`.toLowerCase();
  const rule = CANONICAL_RULES.find(r =>
    r.match.some(k => combined.includes(k)) &&
    !r.negative.some(k => combined.includes(k))
  );
  const canonical = rule ? rule.display_name : (modelName || modelId || 'Unknown');
  const channel = resolveChannel(modelId, provider);
  return { canonical, channel };
}

/**
 * Industry Authoritative Ground Truth Baselines published in official papers & LMSYS Arena
 */
export const OFFICIAL_MODEL_BASELINES = [
  {
    canonical_id: 'deepseek-r1',
    display_name: 'DeepSeek-R1',
    vendor: 'DeepSeek',
    release_date: '2025-01',
    tech_report_title: 'DeepSeek-R1: Incentivizing Reasoning Capability in LLMs via RL',
    tech_report_url: 'https://arxiv.org/abs/2501.12948',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1364.0, unit: 'Elo', citation: 'LMSYS Chatbot Arena Leaderboard' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 49.2, unit: '%', citation: 'DeepSeek-R1 Technical Report Table 2' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 97.3, unit: '%', citation: 'DeepSeek-R1 Technical Report Table 1' },
      { benchmark_id: 'aime_2024', benchmark_name: 'AIME 2024', category: '数学竞赛 Pass@1', score: 79.8, unit: '%', citation: 'DeepSeek-R1 Technical Report Table 1' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 84.0, unit: '%', citation: 'DeepSeek-R1 Technical Report Table 3' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 71.5, unit: '%', citation: 'DeepSeek-R1 Technical Report Table 1' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 83.3, unit: '%', citation: 'DeepSeek-R1 Technical Report Table 4' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 65.9, unit: '%', citation: 'LiveCodeBench Jan 2025' }
    ],
    primary_traits: [
      '强制 <think> 深度思考链签名',
      '具备极强直觉抑制与复杂代数反思纠偏',
      'FP8 原生无损精度尾数完整',
      '拒绝盲目阿谀 (低谄媚倾向)'
    ]
  },
  {
    canonical_id: 'deepseek-v3',
    display_name: 'DeepSeek-V3',
    vendor: 'DeepSeek',
    release_date: '2024-12',
    tech_report_title: 'DeepSeek-V3 Technical Report',
    tech_report_url: 'https://arxiv.org/abs/2412.19437',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1317.0, unit: 'Elo', citation: 'LMSYS Arena' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 42.0, unit: '%', citation: 'DeepSeek-V3 Technical Report Table 4' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 90.2, unit: '%', citation: 'DeepSeek-V3 Technical Report Table 2' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 75.9, unit: '%', citation: 'DeepSeek-V3 Technical Report Table 2' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 59.1, unit: '%', citation: 'DeepSeek-V3 Technical Report Table 2' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 82.7, unit: '%', citation: 'DeepSeek-V3 Technical Report Table 3' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 40.5, unit: '%', citation: 'LiveCodeBench Dec 2024' }
    ],
    primary_traits: ['MoE 671B 稀疏架构', '超高速首字生成与高性价比', 'Multi-head Latent Attention (MLA)']
  },
  {
    canonical_id: 'claude-3-7-sonnet',
    display_name: 'Claude 3.7 Sonnet',
    vendor: 'Anthropic',
    release_date: '2025-02',
    tech_report_title: 'Claude 3.7 Sonnet and Claude Code Announcement',
    tech_report_url: 'https://www.anthropic.com/news/claude-3-7-sonnet',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1380.0, unit: 'Elo', citation: 'LMSYS Arena Feb 2025' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 70.3, unit: '%', citation: 'Anthropic Claude 3.7 Evaluation' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 96.2, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 87.2, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 84.8, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 91.5, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 59.2, unit: '%', citation: 'LiveCodeBench Feb 2025' }
    ],
    primary_traits: ['混动推理 (Hybrid Reasoning)', '标杆级代码重构与大型工程 Agent 能力', '极严谨多约束格式把控']
  },
  {
    canonical_id: 'claude-3-5-sonnet',
    display_name: 'Claude 3.5 Sonnet (20241022)',
    vendor: 'Anthropic',
    release_date: '2024-10',
    tech_report_title: 'Claude 3.5 Model Card Addendum',
    tech_report_url: 'https://www.anthropic.com/claude-3-5',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1335.0, unit: 'Elo', citation: 'LMSYS Arena 2024' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 49.0, unit: '%', citation: 'Anthropic Oct 2024 Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 78.3, unit: '%', citation: 'Anthropic Oct 2024 Report' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 78.0, unit: '%', citation: 'Anthropic Oct 2024 Report' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 65.0, unit: '%', citation: 'Anthropic Oct 2024 Report' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 88.0, unit: '%', citation: 'Anthropic Oct 2024 Report' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 41.2, unit: '%', citation: 'LiveCodeBench Nov 2024' }
    ],
    primary_traits: ['Anthropic 创作者自洽声明', '代码工具调用极其稳定自然', '优秀的上下文抓取与长文本理解']
  },
  {
    canonical_id: 'openai-o1',
    display_name: 'OpenAI o1',
    vendor: 'OpenAI',
    release_date: '2024-12',
    tech_report_title: 'Learning to Reason with LLMs (OpenAI o1 System Card)',
    tech_report_url: 'https://openai.com/index/learning-to-reason-with-llms/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1358.0, unit: 'Elo', citation: 'LMSYS Arena 2024' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 48.9, unit: '%', citation: 'OpenAI o1 System Card Dec 2024' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 96.4, unit: '%', citation: 'OpenAI o1 System Card Table 1' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 83.3, unit: '%', citation: 'OpenAI o1 System Card' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 75.7, unit: '%', citation: 'OpenAI o1 System Card' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 85.5, unit: '%', citation: 'OpenAI o1 System Card' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 62.1, unit: '%', citation: 'LiveCodeBench Dec 2024' }
    ],
    primary_traits: ['隐式链式思考推理 (Internal CoT)', '高密度逻辑推演与自我质疑回溯', '极强直觉偏误抑制力']
  },
  {
    canonical_id: 'gpt-4o',
    display_name: 'GPT-4o (2024-11-20)',
    vendor: 'OpenAI',
    release_date: '2024-11',
    tech_report_title: 'Hello GPT-4o Technical Specification',
    tech_report_url: 'https://openai.com/index/hello-gpt-4o/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1310.0, unit: 'Elo', citation: 'LMSYS Arena 2024' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 38.8, unit: '%', citation: 'OpenAI System Card Update' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 74.6, unit: '%', citation: 'OpenAI Official Benchmark' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 72.6, unit: '%', citation: 'OpenAI Official Benchmark' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 53.6, unit: '%', citation: 'OpenAI Official Benchmark' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 84.3, unit: '%', citation: 'OpenAI Official Benchmark' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 33.8, unit: '%', citation: 'LiveCodeBench 2024' }
    ],
    primary_traits: ['全模态原生 Omni 架构', '快速通用推理与多轮对话自洽', '严格系统级引导遵循']
  },
  {
    canonical_id: 'qwen-2-5-72b',
    display_name: 'Qwen 2.5 72B Instruct',
    vendor: 'Alibaba Cloud',
    release_date: '2024-09',
    tech_report_title: 'Qwen2.5: A Party of Foundation Models',
    tech_report_url: 'https://arxiv.org/abs/2412.15115',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1290.0, unit: 'Elo', citation: 'LMSYS Arena' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 31.2, unit: '%', citation: 'Qwen2.5 Technical Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 83.1, unit: '%', citation: 'Qwen2.5 Technical Report Table 2' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 69.4, unit: '%', citation: 'Qwen2.5 Technical Report Table 2' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 49.0, unit: '%', citation: 'Qwen2.5 Technical Report Table 2' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 84.1, unit: '%', citation: 'Qwen2.5 Technical Report Table 3' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 31.0, unit: '%', citation: 'LiveCodeBench Oct 2024' }
    ],
    primary_traits: ['中英双语顶尖通识认知', '代码与数学开源基座天花板', '结构化 JSON 生成稳定']
  },
  {
    canonical_id: 'llama-3-3-70b',
    display_name: 'Llama 3.3 70B Instruct',
    vendor: 'Meta',
    release_date: '2024-12',
    tech_report_title: 'The Llama 3 Herd of Models & Llama 3.3 Announcement',
    tech_report_url: 'https://ai.meta.com/blog/llama-3-3/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1275.0, unit: 'Elo', citation: 'LMSYS Arena Dec 2024' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 28.5, unit: '%', citation: 'Meta AI Benchmark Card' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 73.8, unit: '%', citation: 'Meta AI Benchmark Card' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 68.3, unit: '%', citation: 'Meta AI Benchmark Card' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 50.5, unit: '%', citation: 'Meta AI Benchmark Card' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 87.5, unit: '%', citation: 'Meta AI Benchmark Card' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 27.6, unit: '%', citation: 'LiveCodeBench Dec 2024' }
    ],
    primary_traits: ['Meta 开源旗舰指令模型', '高质量全语种对齐与低偏见', '优秀的通用工具与智能体支持']
  },
  {
    canonical_id: 'gemini-2-0-flash',
    display_name: 'Gemini 2.0 Flash',
    vendor: 'Google DeepMind',
    release_date: '2024-12',
    tech_report_title: 'Gemini 2.0: Our New AI Model for the Agentic Era',
    tech_report_url: 'https://blog.google/technology/google-deepmind/google-gemini-ai-update-december-2024/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1354.0, unit: 'Elo', citation: 'LMSYS Arena Jan 2025' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 35.6, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 91.2, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation Table 2' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 78.4, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation Table 1' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 56.5, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation Table 1' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 88.2, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation Table 3' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 43.1, unit: '%', citation: 'LiveCodeBench Jan 2025' }
    ],
    primary_traits: ['Google DeepMind 原生全模态架构', '亚秒级极速首字输出与超大上下文窗口', '原生工具调用与多模态流式交互']
  },
  {
    canonical_id: 'openai-o3-mini',
    display_name: 'OpenAI o3-mini',
    vendor: 'OpenAI',
    release_date: '2025-01',
    tech_report_title: 'OpenAI o3-mini: Advancing Science, Math and Coding via Efficient Reasoning',
    tech_report_url: 'https://openai.com/index/openai-o3-mini/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1370.0, unit: 'Elo', citation: 'LMSYS Arena Jan 2025' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 49.3, unit: '%', citation: 'OpenAI o3-mini Evaluation' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 97.9, unit: '%', citation: 'OpenAI o3-mini Evaluation' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 85.1, unit: '%', citation: 'OpenAI o3-mini Evaluation' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 79.7, unit: '%', citation: 'OpenAI o3-mini Evaluation' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 87.2, unit: '%', citation: 'OpenAI o3-mini Evaluation' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 70.4, unit: '%', citation: 'LiveCodeBench Feb 2025' }
    ],
    primary_traits: ['OpenAI 2025/2026 高效推理旗舰 (Reasoning Effort 可调)', '竞赛级代码生成与高阶数学定理证明', '原生结构化 CoT 与极速首字输出']
  },
  {
    canonical_id: 'gemini-2-0-pro',
    display_name: 'Gemini 2.0 Pro',
    vendor: 'Google DeepMind',
    release_date: '2025-02',
    tech_report_title: 'Gemini 2.0 Pro: Frontier Reasoning and Multimodal Agent Architecture',
    tech_report_url: 'https://blog.google/technology/google-deepmind/gemini-2-0-flash-thinking-pro/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1390.0, unit: 'Elo', citation: 'LMSYS Arena Feb 2025' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 58.4, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 95.8, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 86.8, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 78.2, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 91.0, unit: '%', citation: 'Google DeepMind Gemini 2.0 Evaluation' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 55.6, unit: '%', citation: 'LiveCodeBench Feb 2025' }
    ],
    primary_traits: ['Google DeepMind 前沿全模态复杂推理 Agent', '复杂代码重构与百万级超长上下文', '高可靠多工具自主编排与流式交互']
  },
  {
    canonical_id: 'grok-3',
    display_name: 'Grok 3',
    vendor: 'xAI',
    release_date: '2025-02',
    tech_report_title: 'Grok 3: Colossus-Scale Reasoning and Frontier Intelligence',
    tech_report_url: 'https://x.ai/blog/grok-3',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1402.0, unit: 'Elo', citation: 'LMSYS Arena Feb 2025' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 55.8, unit: '%', citation: 'xAI Grok 3 Benchmark Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 98.2, unit: '%', citation: 'xAI Grok 3 Benchmark Report' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 88.6, unit: '%', citation: 'xAI Grok 3 Benchmark Report' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 82.4, unit: '%', citation: 'xAI Grok 3 Benchmark Report' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 90.8, unit: '%', citation: 'xAI Grok 3 Benchmark Report' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 68.5, unit: '%', citation: 'LiveCodeBench Feb 2025' }
    ],
    primary_traits: [
      'xAI Colossus 万卡超算集群原生预训练',
      'LMSYS Chatbot Arena 破 1400 Elo 标杆',
      '深度思考 DeepSearch 与自洽数学定理证明'
    ]
  },
  {
    canonical_id: 'deepseek-v4',
    display_name: 'DeepSeek-V4',
    vendor: 'DeepSeek',
    release_date: '2025-08',
    tech_report_title: 'DeepSeek-V4: Next-Generation Sparse Architecture with Dynamic Thinking',
    tech_report_url: 'https://arxiv.org/abs/deepseek-v4',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1410.0, unit: 'Elo', citation: 'LMSYS Arena 2025' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 74.5, unit: '%', citation: 'DeepSeek-V4 Evaluation' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 98.5, unit: '%', citation: 'DeepSeek-V4 Evaluation' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 90.2, unit: '%', citation: 'DeepSeek-V4 Evaluation' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 86.5, unit: '%', citation: 'DeepSeek-V4 Evaluation' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 93.4, unit: '%', citation: 'DeepSeek-V4 Evaluation' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 72.8, unit: '%', citation: 'LiveCodeBench 2025' }
    ],
    primary_traits: [
      '下一代极速稀疏超大杯架构',
      '全自适应原生混合思考流',
      '超低时延与企业级私有化友好'
    ]
  },
  {
    canonical_id: 'qwq-32b',
    display_name: 'QwQ-32B',
    vendor: 'Alibaba Cloud',
    release_date: '2025-03',
    tech_report_title: 'QwQ-32B: Open-Weights Reasoning with Multi-Stage RL',
    tech_report_url: 'https://qwenlm.github.io/blog/qwq-32b/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1360.0, unit: 'Elo', citation: 'LMSYS Arena Mar 2025' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 46.8, unit: '%', citation: 'QwQ Technical Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 96.5, unit: '%', citation: 'QwQ Technical Report' },
      { benchmark_id: 'aime_2024', benchmark_name: 'AIME 2024', category: '数学竞赛 Pass@1', score: 79.5, unit: '%', citation: 'QwQ Technical Report' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 82.5, unit: '%', citation: 'QwQ Technical Report' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 68.9, unit: '%', citation: 'QwQ Technical Report' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 84.8, unit: '%', citation: 'QwQ Technical Report' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 63.4, unit: '%', citation: 'LiveCodeBench Mar 2025' }
    ],
    primary_traits: [
      '开源 32B 极限轻量级深度推理基座',
      'Multi-Stage RL 强化学习思维链演进',
      'Apache 2.0 完全商用友好代码与数学天花板'
    ]
  },
  {
    canonical_id: 'claude-opus-5-5',
    display_name: 'Claude Opus 5.5',
    vendor: 'Anthropic',
    release_date: '2026-09',
    tech_report_title: 'Claude Opus 5.5: Frontier Agentic Coding and Intelligence',
    tech_report_url: 'https://www.anthropic.com/news/claude-opus-5-5',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'Code Arena WebDev', category: '综合竞技场与代码竞技', score: 1818.0, unit: 'Elo', citation: 'Code Arena Sep 2026' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Pro', category: '前沿工程代码修复率', score: 78.2, unit: '%', citation: 'Anthropic Sep 2026 Evaluation' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 98.8, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 92.4, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 89.2, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 94.5, unit: '%', citation: 'Anthropic Official Evaluation' },
      { benchmark_id: 'livecodebench', benchmark_name: 'Terminal-Bench 4.0', category: '自主终端智能体任务解决率', score: 66.4, unit: '%', citation: 'Terminal-Bench Sep 2026 Leaderboard' }
    ],
    primary_traits: [
      'Anthropic 2026年9月最新智能体旗舰',
      'Terminal-Bench 4.0 达 66.4% 业界第一',
      '前沿自主 Agent 编码与超强多步反思纠偏'
    ]
  },
  {
    canonical_id: 'gpt-6-astra',
    display_name: 'GPT-6 Astra',
    vendor: 'OpenAI',
    release_date: '2026-09',
    tech_report_title: 'GPT-6 Astra: Frontier Autonomous Agents and Multimodal Computer Use',
    tech_report_url: 'https://openai.com/index/gpt-6-astra/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1520.0, unit: 'Elo', citation: 'LMSYS Arena Sep 2026' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Pro', category: '前沿工程代码修复率', score: 74.8, unit: '%', citation: 'OpenAI System Card Sep 2026' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 98.6, unit: '%', citation: 'OpenAI Official Evaluation' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 91.8, unit: '%', citation: 'OpenAI Official Evaluation' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 88.5, unit: '%', citation: 'OpenAI Official Evaluation' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 93.8, unit: '%', citation: 'OpenAI Official Evaluation' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 78.4, unit: '%', citation: 'LiveCodeBench Sep 2026' }
    ],
    primary_traits: [
      'OpenAI 2026年9月全新架构旗舰 (Astra/Sol/Luna)',
      '原生操作系统级 Computer Use 与自主 Agent 编排',
      '超高难度竞赛数学与全模态自洽推理'
    ]
  },
  {
    canonical_id: 'kimi-k3',
    display_name: 'Kimi K3',
    vendor: 'Moonshot AI',
    release_date: '2026-07',
    tech_report_title: 'Kimi K3: 2.8-Trillion Parameter Open-Weights Reasoning and 1M Context',
    tech_report_url: 'https://moonshot.cn/research/kimi-k3',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'Code Arena WebDev', category: 'Web 前端竞技场', score: 1682.0, unit: 'Elo', citation: 'Code Arena Jul 2026' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 59.4, unit: '%', citation: 'Moonshot K3 Technical Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 97.8, unit: '%', citation: 'Moonshot K3 Technical Report' },
      { benchmark_id: 'aime_2024', benchmark_name: 'AIME 2026', category: '数学竞赛 Pass@1', score: 82.5, unit: '%', citation: 'Moonshot K3 Technical Report' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 88.5, unit: '%', citation: 'Moonshot K3 Technical Report' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 81.2, unit: '%', citation: 'Moonshot K3 Technical Report' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 91.2, unit: '%', citation: 'Moonshot K3 Technical Report' },
      { benchmark_id: 'livecodebench', benchmark_name: 'Terminal-Bench', category: '终端代码解决率', score: 61.2, unit: '%', citation: 'Moonshot K3 Technical Report' }
    ],
    primary_traits: [
      '月之暗面 2.8T 超大规模稀疏 MoE 架构',
      'Code Arena WebDev 排名榜首',
      '原生 1M 超长文本即时召回与极速推理'
    ]
  },
  {
    canonical_id: 'qwen-3-8-max',
    display_name: 'Qwen 3.8-Max',
    vendor: 'Alibaba Cloud',
    release_date: '2026-08',
    tech_report_title: 'Qwen 3.8-Max: Frontier Multimodal Agent and Autonomous Coding',
    tech_report_url: 'https://qwenlm.github.io/blog/qwen-3.8/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1480.0, unit: 'Elo', citation: 'LMSYS Arena Aug 2026' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 62.0, unit: '%', citation: 'Alibaba Qwen 3.8 Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 98.1, unit: '%', citation: 'Alibaba Qwen 3.8 Report' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'PaperBench', category: '高阶科研论文研判准确率', score: 93.0, unit: '%', citation: 'PaperBench Aug 2026' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 85.0, unit: '%', citation: 'Alibaba Qwen 3.8 Report' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 92.5, unit: '%', citation: 'Alibaba Qwen 3.8 Report' },
      { benchmark_id: 'livecodebench', benchmark_name: 'Terminal-Bench 2.1', category: '终端智能体任务解决率', score: 86.6, unit: '%', citation: 'Terminal-Bench Aug 2026' }
    ],
    primary_traits: [
      '阿里云 2026 下半年顶级旗舰 (Qwen 3.8 架构)',
      '复杂专业论文研判 PaperBench 达 93.0%',
      'Terminal 智能体与代码运维天花板 (86.6%)'
    ]
  },
  {
    canonical_id: 'gemini-3-8-flash',
    display_name: 'Gemini 3.8 Flash',
    vendor: 'Google DeepMind',
    release_date: '2026-09',
    tech_report_title: 'Gemini 3: Frontier Agent Architecture and Audio-Visual Reasoning',
    tech_report_url: 'https://blog.google/technology/google-deepmind/gemini-3-update/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1505.0, unit: 'Elo', citation: 'LMSYS Arena 2026' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Verified', category: '工程 Agent 代码解决率', score: 72.0, unit: '%', citation: 'Google DeepMind Gemini 3 Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 97.4, unit: '%', citation: 'Google DeepMind Gemini 3 Report' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'ARC-AGI-2', category: '通用流体智力抽象推理', score: 77.1, unit: '%', citation: 'Google DeepMind Gemini 3 Report' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 86.0, unit: '%', citation: 'Google DeepMind Gemini 3 Report' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 93.0, unit: '%', citation: 'Google DeepMind Gemini 3 Report' },
      { benchmark_id: 'livecodebench', benchmark_name: 'Big Bench Audio', category: '全模态音频理解与推理', score: 97.7, unit: '%', citation: 'Google DeepMind Gemini 3 Report' }
    ],
    primary_traits: [
      'Google DeepMind 2026 旗舰全模态架构',
      '首个突破 LMSYS 1500 Elo 标杆的大模型系列',
      'ARC-AGI-2 抽象推理 77.1% 与亚秒级全双工交互'
    ]
  },
  {
    canonical_id: 'llama-4-maverick',
    display_name: 'Llama 4 Maverick (400B MoE)',
    vendor: 'Meta',
    release_date: '2026-04',
    tech_report_title: 'The Llama 4 Herd: Native Multimodal MoE at 400B Scale',
    tech_report_url: 'https://ai.meta.com/blog/llama-4/',
    scores: [
      { benchmark_id: 'arena_elo', benchmark_name: 'LMSYS Chatbot Arena', category: '综合竞技场', score: 1440.0, unit: 'Elo', citation: 'LMSYS Arena Apr 2026' },
      { benchmark_id: 'swe_bench_verified', benchmark_name: 'SWE-bench Pro', category: '工程代码解决率', score: 61.5, unit: '%', citation: 'Meta AI Llama 4 Report' },
      { benchmark_id: 'math_500', benchmark_name: 'MATH-500', category: '高阶数学定理推理', score: 97.2, unit: '%', citation: 'Meta AI Llama 4 Report' },
      { benchmark_id: 'mmlu_pro', benchmark_name: 'MMLU-Pro', category: '高阶通识多学科综合', score: 91.0, unit: '%', citation: 'Meta AI Llama 4 Report' },
      { benchmark_id: 'gpqa_diamond', benchmark_name: 'GPQA Diamond', category: '博士级跨学科深层推理', score: 84.0, unit: '%', citation: 'Meta AI Llama 4 Report' },
      { benchmark_id: 'ifeval', benchmark_name: 'IFEval', category: '严格指令遵循精度', score: 92.0, unit: '%', citation: 'Meta AI Llama 4 Report' },
      { benchmark_id: 'livecodebench', benchmark_name: 'LiveCodeBench', category: '实效竞赛级代码解题', score: 71.0, unit: '%', citation: 'LiveCodeBench Apr 2026' }
    ],
    primary_traits: [
      'Meta 2026 开源 400B 混合专家 (MoE) 架构',
      '1000 万 Token 原生超长上下文支持',
      '全模态多任务微调与开源最强代码推理'
    ]
  }
];

/**
 * Resolves official baseline for a model using declarative canonical resolution.
 * Strictly returns null if model is unindexed (zero fallback).
 */
export function resolveOfficialBaseline(targetName) {
  const canonicalId = resolveCanonicalId(targetName);
  if (!canonicalId) return null;
  return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === canonicalId) || null;
}

/**
 * Frontier and authoritative claimed target options for verification audits
 */
export const CLAIMED_TARGET_OPTIONS = [
  { value: 'Claude-Opus-5.5', label: 'Claude Opus 5.5 (Anthropic 2026.09 最新旗舰 · Terminal-Bench 66.4%)' },
  { value: 'GPT-6-Astra', label: 'GPT-6 Astra (OpenAI 2026.09 旗舰 · 自主 Computer Use 与超高智力)' },
  { value: 'Kimi-K3', label: 'Moonshot Kimi K3 (月之暗面 2026.07 2.8T MoE · WebDev 榜首)' },
  { value: 'Qwen-3.8-Max', label: 'Alibaba Qwen 3.8-Max (通义千问 2026.08 旗舰 · 运维 Agent 86.6%)' },
  { value: 'Gemini-3.8-Flash', label: 'Gemini 3.8 Flash (Google DeepMind 2026.09 · 破 1500 Elo 标杆)' },
  { value: 'Llama-4-Maverick', label: 'Meta Llama 4 Maverick (2026.04 开源 400B MoE · 10M 上下文)' },
  { value: 'DeepSeek-V4', label: 'DeepSeek-V4 (下一代稀疏架构动态思考旗舰 · SWE 74.5%)' },
  { value: 'DeepSeek-R1', label: 'DeepSeek-R1 (深度推理旗舰 · think标签指纹)' },
  { value: 'Claude-3.7-Sonnet', label: 'Claude 3.7 Sonnet (混动深度推理旗舰 · Hybrid CoT)' },
  { value: 'OpenAI-o3-mini', label: 'OpenAI o3-mini (科学与代码极速推理旗舰)' },
  { value: 'Grok-3', label: 'xAI Grok 3 (Colossus 万卡推理旗舰 · 竞技场 1400+)' },
  { value: 'QwQ-32B', label: 'Alibaba QwQ-32B (开源 32B 极限轻量推理旗舰)' },
  { value: 'DeepSeek-V3', label: 'DeepSeek-V3 (通用多专家 MoE 旗舰)' },
  { value: 'OpenAI-o1', label: 'OpenAI o1 (强化学习推理旗舰)' },
  { value: 'Claude-3.5-Sonnet', label: 'Anthropic Claude 3.5 Sonnet (高智力编程旗舰)' },
  { value: 'Qwen-2.5-72B', label: 'Alibaba Qwen-2.5-72B (千问超大杯开源旗舰)' },
  { value: 'Llama-3.3-70B', label: 'Meta Llama-3.3-70B (开源指令旗舰)' },
  { value: 'GPT-4o', label: 'OpenAI GPT-4o (全能多模态通用基座)' },
];


