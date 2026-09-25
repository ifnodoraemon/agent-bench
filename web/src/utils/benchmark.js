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

/**
 * Resolve difficulty tier (L1 to L5)
 */
export function resolveDifficulty(c) {
  if (c.difficulty && c.difficulty.trim() !== '') {
    const d = c.difficulty.toUpperCase();
    if (TIER_DEFINITIONS[d]) return d;
  }

  const id = (c.test_case_id || c.id || '').toLowerCase();
  if (id.includes('_hard_') || id.includes('putnam') || id.includes('swe_hard') || id.includes('jailbreak')) {
    return 'L5';
  }
  if (
    id.startsWith('agent') ||
    id.startsWith('sec') ||
    id.startsWith('devops') ||
    id.startsWith('swe') ||
    id.startsWith('react') ||
    id.startsWith('tool') ||
    id.startsWith('error')
  ) {
    return 'L4';
  }
  if (
    id.startsWith('med') ||
    id.startsWith('law') ||
    id.startsWith('legal') ||
    id.startsWith('fin') ||
    id.startsWith('math') ||
    ['medical', 'legal', 'finance', 'math_logic'].includes(c.category)
  ) {
    return 'L3';
  }
  if (
    id.startsWith('hum') ||
    id.startsWith('sci') ||
    id.startsWith('multi') ||
    id.startsWith('needle') ||
    id.startsWith('code') ||
    id.startsWith('hallucination')
  ) {
    return 'L2';
  }
  return 'L1';
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

/**
 * Resolves a model's canonical family and provider channel
 */
export function resolveCanonicalModelAndChannel(modelId, modelName, provider) {
  const lowerId = (modelId || '').toLowerCase();
  const lowerName = (modelName || '').toLowerCase();
  const lowerProv = (provider || '').toLowerCase();

  // 1. Resolve Channel
  let channel = '官方直连 (Official)';
  if (lowerId.includes('silicon') || lowerProv.includes('silicon')) {
    channel = '硅基流动 (SiliconFlow)';
  } else if (lowerId.includes('volc') || lowerId.includes('ark') || lowerId.startsWith('ep-') || lowerProv.includes('volc')) {
    channel = '火山方舟 (Volcengine)';
  } else if (lowerId.includes('openrouter') || lowerProv.includes('openrouter')) {
    channel = 'OpenRouter';
  } else if (lowerId.includes('dashscope') || lowerId.includes('bailian') || lowerProv.includes('aliyun')) {
    channel = '阿里云百炼 (DashScope)';
  } else if (lowerId.includes('together') || lowerProv.includes('together')) {
    channel = 'Together AI';
  } else if (lowerId.includes('groq') || lowerProv.includes('groq')) {
    channel = 'Groq (LPU)';
  } else if (lowerId.includes('bedrock') || lowerProv.includes('bedrock')) {
    channel = 'AWS Bedrock';
  } else if (lowerId.includes('vertex') || lowerProv.includes('vertex')) {
    channel = 'GCP Vertex AI';
  } else if (lowerId.includes('azure') || lowerProv.includes('azure')) {
    channel = 'Azure AI Foundry';
  } else if (lowerId.includes('vllm') || lowerId.includes('sglang') || lowerId.includes('local') || lowerId.includes('ollama')) {
    channel = '私有集群 (Self-Hosted/vLLM)';
  } else if (lowerId.startsWith('mock')) {
    channel = '模拟沙盒 (Mock Sandbox)';
  }

  // 2. Resolve Canonical Model Family
  let canonical = modelName || modelId;
  if (lowerId.includes('deepseek-r1') || lowerId.includes('deepseek-reasoner') || lowerName.includes('deepseek-r1') || lowerName.includes('r1')) {
    canonical = 'DeepSeek-R1';
  } else if (lowerId.includes('deepseek-v3') || lowerId.includes('deepseek-chat') || lowerName.includes('deepseek-v3') || lowerName.includes('deepseek-v4')) {
    canonical = 'DeepSeek-V3';
  } else if (lowerId.includes('claude-3-5-sonnet') || lowerName.includes('claude-3.5-sonnet')) {
    canonical = 'Claude-3.5-Sonnet';
  } else if (lowerId.includes('gpt-4o-mini') || lowerName.includes('gpt-4o-mini')) {
    canonical = 'GPT-4o-mini';
  } else if (lowerId.includes('gpt-4o') || lowerName.includes('gpt-4o')) {
    canonical = 'GPT-4o';
  } else if (lowerId.includes('qwen2.5-72b') || lowerId.includes('qwen-2.5-72b') || lowerName.includes('qwen2.5-72b') || lowerName.includes('qwen3.8')) {
    canonical = 'Qwen-2.5-72B';
  } else if (lowerId.includes('glm-4') || lowerName.includes('glm-5') || lowerName.includes('glm')) {
    canonical = 'GLM-4 / GLM-5';
  } else if (lowerId.includes('llama-3.3-70b') || lowerId.includes('llama3.3:70b') || lowerName.includes('llama-3.3')) {
    canonical = 'Llama-3.3-70B';
  } else if (lowerId.startsWith('mock-pro')) {
    canonical = 'Mock-Pro-v1';
  } else if (lowerId.startsWith('mock-fast')) {
    canonical = 'Mock-Fast-v1';
  }

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
  }
];

export function resolveOfficialBaseline(targetName) {
  if (!targetName) return null;
  const lower = targetName.toLowerCase();
  if (lower.includes('deepseek') && (lower.includes('r1') || lower.includes('reasoner'))) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'deepseek-r1');
  }
  if (lower.includes('deepseek') && (lower.includes('v3') || lower.includes('chat'))) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'deepseek-v3');
  }
  if (lower.includes('claude') && (lower.includes('3.7') || lower.includes('3-7'))) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'claude-3-7-sonnet');
  }
  if (lower.includes('claude') && (lower.includes('3.5') || lower.includes('3-5') || lower.includes('sonnet'))) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'claude-3-5-sonnet');
  }
  if (lower.includes('o1')) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'openai-o1');
  }
  if (lower.includes('gpt-4o') || lower.includes('gpt4o')) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'gpt-4o');
  }
  if (lower.includes('qwen')) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'qwen-2-5-72b');
  }
  if (lower.includes('llama')) {
    return OFFICIAL_MODEL_BASELINES.find(b => b.canonical_id === 'llama-3-3-70b');
  }
  return OFFICIAL_MODEL_BASELINES[0]; // fallback to DeepSeek-R1 for mock
}

