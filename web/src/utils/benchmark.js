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
  { channel: '火山方舟 (Volcengine)', match: ['volc', 'ep-'] },
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
  // Variant-specific families first.
  {
    canonical_id: 'deepseek-v4-flash',
    display_name: 'DeepSeek-V4-Flash',
    match: ['deepseek-v4-flash', 'deepseek-v4_flash'],
    negative: []
  },
  {
    canonical_id: 'qwen-3-8-27b',
    display_name: 'Qwen-3.8-27B',
    match: ['qwen3.8-27b', 'qwen-3.8-27b', 'qwen3.8_27b'],
    negative: []
  },
  {
    canonical_id: 'glm-5-2-fp8',
    display_name: 'GLM-5.2-FP8',
    match: ['glm-5.2-fp8', 'glm-5-2-fp8', 'glm5.2-fp8'],
    negative: []
  },
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
    canonical_id: 'gpt-5',
    display_name: 'GPT-5',
    match: ['gpt-5'],
    negative: ['gpt-6']
  },
  {
    canonical_id: 'qwen-3-8-max',
    display_name: 'Qwen-3.8-Max',
    match: ['qwen-3.8-max', 'qwen-3.8', 'qwen3.8'],
    negative: ['qwq']
  },
  {
    canonical_id: 'gemini-3-8-flash',
    display_name: 'Gemini-3.8-Flash',
    match: ['gemini-3.8-flash', 'gemini-3.8', 'gemini-3.1'],
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
    match: ['deepseek-r1', 'deepseek_r1', 'deepseek-reasoner', 'r1-distill'],
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
    match: ['grok-3', 'grok3', 'grok-4', 'grok4'],
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
    match: ['o3-mini', 'o3mini', 'openai-o3'],
    negative: []
  },
  {
    canonical_id: 'openai-o1',
    display_name: 'OpenAI-o1',
    match: ['o1-preview', 'o1-mini', 'openai-o1'],
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
    match: ['gpt-4o', 'gpt4o'],
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
    match: ['glm-5.2', 'glm5.2', 'glm-5', 'glm-4'],
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


function aliasMatches(model, keyword) {
  const m = String(model || '').toLowerCase();
  const k = String(keyword || '').toLowerCase();
  return m === k
    || m.startsWith(k + '-')
    || m.startsWith(k + '_')
    || m.startsWith(k + '.')
    || m.includes('/' + k)
    || m.includes('/' + k + '-');
}

export function resolveCanonicalId(rawName) {
  if (!rawName) return null;
  const lower = rawName.toLowerCase();
  const rule = CANONICAL_RULES.find(r =>
    r.match.some(k => aliasMatches(lower, k)) &&
    !r.negative.some(k => aliasMatches(lower, k))
  );
  return rule ? rule.canonical_id : null;
}

/**
 * Resolves a model's canonical family and provider channel using declarative rule tables
 */
export function resolveCanonicalModelAndChannel(modelId, modelName, provider) {
  const combined = `${modelId || ''} ${modelName || ''}`.toLowerCase();
  const rule = CANONICAL_RULES.find(r =>
    r.match.some(k => aliasMatches(combined, k)) &&
    !r.negative.some(k => aliasMatches(combined, k))
  );
  const canonical = rule ? rule.display_name : (modelName || modelId || 'Unknown');
  const channel = resolveChannel(modelId, provider);
  return { canonical, channel };
}

/**
 * Industry Authoritative Ground Truth Baselines published in official papers & LMSYS Arena
 */
/**
 * Resolves official baseline for a model using declarative canonical resolution.
 * Strictly returns null if model is unindexed (zero fallback).
 */
/**
 * Frontier and authoritative claimed target options for verification audits
 */
export const CLAIMED_TARGET_OPTIONS = [
  { value: 'Claude-Opus-5.5', label: 'Claude Opus 5.5 (Anthropic 标称型号；基准分待核实)' },
  { value: 'GPT-6-Astra', label: 'GPT-6 Astra (OpenAI 标称型号；基准分待核实)' },
  { value: 'Kimi-K3', label: 'Moonshot Kimi K3 (标称型号；基准分待核实)' },
  { value: 'Qwen-3.8-Max', label: 'Alibaba Qwen 3.8-Max (标称型号；基准分待核实)' },
  { value: 'Gemini-3.8-Flash', label: 'Gemini 3.8 Flash (标称型号；基准分待核实)' },
  { value: 'Llama-4-Maverick', label: 'Meta Llama 4 Maverick (标称型号；基准分待核实)' },
  { value: 'DeepSeek-V4', label: 'DeepSeek-V4 (标称型号；基准分待核实)' },
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


