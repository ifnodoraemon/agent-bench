<template>
  <div class="diagnostic-container">
    <!-- Header -->
    <div class="diagnostic-header-card">
      <div class="header-left">
        <h3 class="panel-title">💡 智能体自诊断与评测洞见引擎 (Agentic Diagnostics)</h3>
        <p class="panel-subtitle">
          全自动分析模型在复杂工具编排、长程思维链与垂直任务中的隐性失效模式，提炼深度归因与落地调优策略。
        </p>
      </div>

      <div class="model-picker-wrap">
        <label>选择诊断分析模型:</label>
        <select v-model="selectedModelId" class="diag-model-select">
          <option v-for="m in models" :key="m.model_id" :value="m.model_id">
            {{ m.model_name || m.model_id }}
          </option>
        </select>
      </div>
    </div>

    <!-- Overview Scorecards -->
    <div v-if="currentDiag" class="diag-scorecards-grid">
      <!-- Health Card -->
      <div class="scorecard health-card">
        <div class="scorecard-top">
          <span class="card-icon">🛡️</span>
          <span class="card-lbl">智能体鲁棒性评级</span>
        </div>
        <div class="scorecard-main">
          <span class="grade-badge" :class="'grade-' + currentDiag.healthGrade.toLowerCase()">
            {{ currentDiag.healthGrade }}
          </span>
          <div class="grade-desc">
            <span class="grade-title">{{ currentDiag.healthTitle }}</span>
            <span class="grade-sub">共 {{ currentDiag.totalCases }} 题，成功率 {{ currentDiag.accuracyPercent }}%</span>
          </div>
        </div>
      </div>

      <!-- Thinking Dynamics Card -->
      <div class="scorecard thinking-card">
        <div class="scorecard-top">
          <span class="card-icon">🧠</span>
          <span class="card-lbl">思维链自洽与陷入指数</span>
        </div>
        <div class="scorecard-main">
          <span class="metric-big" :class="currentDiag.hesitationRatio > 2.0 ? 'text-amber' : 'text-emerald'">
            {{ currentDiag.hesitationRatio.toFixed(1) }}x
          </span>
          <div class="grade-desc">
            <span class="grade-title">
              {{ currentDiag.hesitationRatio > 2.0 ? '存在过度犹豫反刍' : '思维效率高度自洽' }}
            </span>
            <span class="grade-sub">失败题均字符 {{ currentDiag.avgFailChars }} vs 通过题 {{ currentDiag.avgPassChars }}</span>
          </div>
        </div>
      </div>

      <!-- Latency Stability Card -->
      <div class="scorecard latency-card">
        <div class="scorecard-top">
          <span class="card-icon">⏱️</span>
          <span class="card-lbl">长尾延迟抖动 (P95 / Avg)</span>
        </div>
        <div class="scorecard-main">
          <span class="metric-big" :class="currentDiag.latencyJitterRatio > 3.0 ? 'text-rose' : 'text-sky'">
            {{ currentDiag.latencyJitterRatio.toFixed(1) }}x
          </span>
          <div class="grade-desc">
            <span class="grade-title">
              {{ currentDiag.latencyJitterRatio > 3.0 ? '高长尾偶发卡顿' : '端到端延迟高度平稳' }}
            </span>
            <span class="grade-sub">P95: {{ Math.round(currentModel.p95_latency_ms || 0) }}ms / Avg: {{ Math.round(currentModel.avg_latency_ms || 0) }}ms</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Deep Analysis Grid: Left Failure Breakdown, Right Vulnerabilities -->
    <div v-if="currentDiag" class="analysis-details-grid">
      <!-- Left: Failure Causes -->
      <div class="detail-card">
        <div class="card-header">
          <h4>❌ 失效根因归类分布 (Failure Attribution)</h4>
          <span class="chart-hint">共记录 {{ currentDiag.failedCount }} 例未通过测试用例</span>
        </div>

        <div class="causes-list">
          <div
            v-for="cause in currentDiag.failureCauses"
            :key="cause.type"
            class="cause-row"
          >
            <div class="cause-info">
              <span class="cause-name">{{ cause.name }}</span>
              <span class="cause-count">{{ cause.count }} 例 ({{ cause.percent }}%)</span>
            </div>
            <div class="cause-track">
              <div
                class="cause-fill"
                :style="{ width: cause.percent + '%', backgroundColor: cause.color }"
              ></div>
            </div>
            <p class="cause-desc">{{ cause.desc }}</p>
          </div>
        </div>
      </div>

      <!-- Right: Weak Categories Radar List -->
      <div class="detail-card">
        <div class="card-header">
          <h4>🚨 垂直领域短板预警 (Vulnerability Radar)</h4>
          <span class="chart-hint">通过率低于 90% 的风险子领域</span>
        </div>

        <div class="weak-list">
          <div
            v-if="currentDiag.weakCategories.length === 0"
            class="all-strong-box"
          >
            <span class="icon">🎉</span>
            <strong>全领域均衡卓越</strong>
            <p>该模型在全部 18 个细分子类中通过率均维持在 90% 以上！</p>
          </div>

          <div
            v-for="weak in currentDiag.weakCategories"
            :key="weak.category"
            class="weak-item"
          >
            <div class="weak-header">
              <span class="weak-cat">{{ weak.name }}</span>
              <span class="badge badge-warning">{{ weak.accuracyPercent }}% 通过率</span>
            </div>
            <p class="weak-advise">{{ weak.advice }}</p>
          </div>
        </div>
      </div>
    </div>

    <!-- Actionable Deployment & Tuning Recommendations -->
    <div v-if="currentDiag" class="recommendations-card">
      <div class="card-header">
        <h4>🎯 生产落地选型与 Prompt / 微调优化建议</h4>
      </div>

      <div class="rec-grid">
        <div
          v-for="(rec, idx) in currentDiag.recommendations"
          :key="idx"
          class="rec-box"
        >
          <div class="rec-top">
            <span class="rec-icon">{{ rec.icon }}</span>
            <strong class="rec-title">{{ rec.title }}</strong>
          </div>
          <p class="rec-body">{{ rec.text }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue';
import { CATEGORY_NAMES, extractThinking } from '../utils/benchmark';

const props = defineProps({
  models: {
    type: Array,
    required: true
  }
});

const selectedModelId = ref(props.models[0]?.model_id || '');

watch(() => props.models, (newModels) => {
  if (newModels.length > 0 && (!selectedModelId.value || !newModels.find(m => m.model_id === selectedModelId.value))) {
    selectedModelId.value = newModels[0].model_id;
  }
}, { immediate: true });

const currentModel = computed(() => {
  return props.models.find(m => m.model_id === selectedModelId.value) || props.models[0];
});

const currentDiag = computed(() => {
  if (!currentModel.value) return null;
  const m = currentModel.value;
  const cases = m.case_results || [];
  const totalCases = cases.length;
  const passedCases = cases.filter(c => c.passed);
  const failedCases = cases.filter(c => !c.passed);
  const acc = m.overall_accuracy || (totalCases > 0 ? passedCases.length / totalCases : 0);

  // Health Grade
  let healthGrade = 'A+';
  let healthTitle = '综合顶尖 · 极强通用与复杂环境抗压能力';
  if (acc < 0.85) {
    healthGrade = 'C';
    healthTitle = '存在明显能力断层 · 需特定对齐微调';
  } else if (acc < 0.92) {
    healthGrade = 'B';
    healthTitle = '表现良好 · 少数极限推理场景存在瓶颈';
  } else if (acc < 0.96) {
    healthGrade = 'A';
    healthTitle = '高水准基准 · 具备工业级自主智能体实力';
  }

  // Thinking dynamics
  let passThinkingLenSum = 0;
  passedCases.forEach(c => {
    const t = extractThinking(c.model_output).thinking;
    passThinkingLenSum += t.length;
  });
  let failThinkingLenSum = 0;
  failedCases.forEach(c => {
    const t = extractThinking(c.model_output).thinking;
    failThinkingLenSum += t.length;
  });

  const avgPassChars = passedCases.length > 0 ? Math.round(passThinkingLenSum / passedCases.length) : 0;
  const avgFailChars = failedCases.length > 0 ? Math.round(failThinkingLenSum / failedCases.length) : 0;
  const hesitationRatio = avgPassChars > 0 ? (avgFailChars / avgPassChars) : 1.0;

  // Latency jitter
  const avgLat = m.avg_latency_ms || 1;
  const p95Lat = m.p95_latency_ms || avgLat;
  const latencyJitterRatio = p95Lat / avgLat;

  // Failure attribution
  let syntaxCount = 0;
  let assertCount = 0;
  let timeoutCount = 0;
  let runtimeErrCount = 0;

  failedCases.forEach(c => {
    const err = (c.error || '').toLowerCase();
    const reason = (c.reason || '').toLowerCase();
    if (err.includes('timeout') || reason.includes('timeout')) {
      timeoutCount++;
    } else if (err.includes('syntax') || reason.includes('json') || reason.includes('schema') || reason.includes('tool')) {
      syntaxCount++;
    } else if (err.length > 0) {
      runtimeErrCount++;
    } else {
      assertCount++;
    }
  });

  const totalFails = failedCases.length || 1;
  const failureCauses = [
    {
      type: 'assert',
      name: '业务逻辑与结果断言未满足',
      count: assertCount,
      percent: Math.round((assertCount / totalFails) * 100),
      color: '#ef4444',
      desc: '模型给出了结果，但在单元测试、数学验算或精确规则匹配上存在数值或逻辑瑕疵。'
    },
    {
      type: 'syntax',
      name: '工具调用格式或 Schema 解析失败',
      count: syntaxCount,
      percent: Math.round((syntaxCount / totalFails) * 100),
      color: '#f59e0b',
      desc: '模型生成的 JSON 参数未闭合、键名缺失或未严格遵循 API 工具函数声明契约。'
    },
    {
      type: 'runtime',
      name: '沙箱命令执行异常 / 异常自愈失败',
      count: runtimeErrCount,
      percent: Math.round((runtimeErrCount / totalFails) * 100),
      color: '#a855f7',
      desc: 'Bash 命令返回非零错误码，且模型未能正确识别 Stderr 并在后续轮次有效回退修复。'
    },
    {
      type: 'timeout',
      name: '多轮长文本或推理耗时超时',
      count: timeoutCount,
      percent: Math.round((timeoutCount / totalFails) * 100),
      color: '#0ea5e9',
      desc: '超过测试用例自适应限定耗时 (60s~600s)，多发生在死循环搜索或长文件大范围重写场景。'
    }
  ].filter(c => c.count > 0);

  // Weak categories (< 90%)
  const weakCategories = [];
  if (m.category_summaries) {
    for (const [k, s] of Object.entries(m.category_summaries)) {
      if (s.accuracy < 0.90) {
        weakCategories.push({
          category: k,
          name: CATEGORY_NAMES[k] || k,
          accuracyPercent: ((s.accuracy || 0) * 100).toFixed(1),
          advice: getCategoryAdvice(k)
        });
      }
    }
  }

  // Recommendations
  const recommendations = [
    {
      icon: '🚀',
      title: '生产部署调度定位',
      text: acc >= 0.94
        ? '适合作为核心长程多步 Agent (如 SWE-Agent、代码自动重构系统) 的首选决策底座模型。'
        : '推荐作为高并发前置意图识别或单轮 RAG 召回摘要节点，高危复杂场景辅以双模型交叉校验。'
    },
    {
      icon: '🛠️',
      title: '提示工程与约束优化建议',
      text: syntaxCount > 0
        ? '在 System Prompt 中追加更严格的 Few-Shot 工具参数声明与“禁止多余解释性包裹”约束，可大幅减少格式漂移。'
        : '系统指令遵循能力稳固，可直接启用紧凑型压缩 Prompt 提升首字响应速度 (TTFT)。'
    },
    {
      icon: '🧠',
      title: '长思维链调优启示',
      text: hesitationRatio > 2.0
        ? '失败题目思考长度显著膨胀，存在循环自证陷阱。建议微调时引入“及早验算剪枝”惩罚机制，避免无效字数堆砌。'
        : '思考过程紧凑自洽，推导与验证边界清晰，具有极佳的思维能效比。'
    }
  ];

  return {
    totalCases,
    failedCount: failedCases.length,
    accuracyPercent: (acc * 100).toFixed(1),
    healthGrade,
    healthTitle,
    hesitationRatio,
    avgPassChars,
    avgFailChars,
    latencyJitterRatio,
    failureCauses,
    weakCategories,
    recommendations
  };
});

function getCategoryAdvice(cat) {
  switch (cat) {
    case 'swe': return '代码修改中建议优先要求模型打印完整 diff，避免全文件重写截断。';
    case 'devops': return '针对容器与网络操作，建议在预置工具中注入更直观的报错解析指导。';
    case 'math_logic': return '建议挂载外部 Python 沙箱验证环境，通过程序辅助验算避免心算偏差。';
    case 'finance': return '对长财报与复杂表格问答，建议强化多表格跨周期计算的思考链提示。';
    case 'safety': return '需进一步强化越狱伪装识别，防范以代码分析、逆向反汇编为伪装的注入漏洞。';
    default: return '建议在该垂直领域追加高质量专有数据集进行 DPO/RLAIF 偏好对齐。';
  }
}
</script>

<style scoped>
.diagnostic-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.diagnostic-header-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem 1.5rem;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.panel-title {
  color: var(--text-heading);
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 0.3rem;
}

.panel-subtitle {
  color: var(--text-muted);
  font-size: 0.85rem;
  max-width: 780px;
}

.model-picker-wrap {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.model-picker-wrap label {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-weight: 700;
}

.diag-model-select {
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  color: var(--text-heading);
  padding: 0.5rem 1rem;
  border-radius: 8px;
  font-size: 0.9rem;
  font-weight: 700;
  outline: none;
}

/* Scorecards */
.diag-scorecards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 1rem;
}

.scorecard {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.15rem 1.35rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.scorecard-top {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.card-icon {
  font-size: 1.2rem;
}

.card-lbl {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--text-muted);
}

.scorecard-main {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.grade-badge {
  font-family: var(--font-mono);
  font-size: 2.2rem;
  font-weight: 900;
  line-height: 1;
  padding: 0.2rem 0.6rem;
  border-radius: 8px;
}

.grade-a\+ { background: rgba(16, 185, 129, 0.15); color: #10b981; }
.grade-a { background: rgba(14, 165, 233, 0.15); color: #0ea5e9; }
.grade-b { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
.grade-c { background: rgba(244, 63, 94, 0.15); color: #f43f5e; }

.grade-desc {
  display: flex;
  flex-direction: column;
}

.grade-title {
  color: var(--text-heading);
  font-weight: 700;
  font-size: 0.88rem;
}

.grade-sub {
  color: var(--text-faint);
  font-size: 0.75rem;
  margin-top: 0.15rem;
}

.metric-big {
  font-family: var(--font-mono);
  font-size: 2rem;
  font-weight: 900;
  line-height: 1;
}

.text-emerald { color: #10b981; }
.text-amber { color: #f59e0b; }
.text-sky { color: #0ea5e9; }
.text-rose { color: #f43f5e; }

/* Analysis Grid */
.analysis-details-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
}

@media (max-width: 960px) {
  .analysis-details-grid {
    grid-template-columns: 1fr;
  }
}

.detail-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
}

.card-header {
  margin-bottom: 1rem;
}

.card-header h4 {
  color: var(--text-heading);
  font-size: 1.05rem;
  font-weight: 700;
}

.chart-hint {
  font-size: 0.78rem;
  color: var(--text-faint);
}

.causes-list {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.cause-row {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  background: var(--bg-surface);
  border-radius: 8px;
  padding: 0.75rem 0.9rem;
  border: 1px solid var(--border-soft);
}

.cause-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.cause-name {
  font-size: 0.82rem;
  font-weight: 700;
  color: var(--text-heading);
}

.cause-count {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-weight: 600;
}

.cause-track {
  height: 6px;
  background: var(--bg-subtle);
  border-radius: 3px;
  overflow: hidden;
  margin: 0.2rem 0;
}

.cause-fill {
  height: 100%;
  border-radius: 3px;
}

.cause-desc {
  font-size: 0.75rem;
  color: var(--text-muted);
  line-height: 1.35;
}

.weak-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.all-strong-box {
  background: rgba(16, 185, 129, 0.08);
  border: 1px dashed var(--accent-emerald);
  border-radius: 8px;
  padding: 2rem 1.5rem;
  text-align: center;
  color: var(--accent-emerald);
}

.all-strong-box .icon {
  font-size: 2rem;
  display: block;
  margin-bottom: 0.4rem;
}

.all-strong-box p {
  color: var(--text-muted);
  font-size: 0.8rem;
  margin-top: 0.2rem;
}

.weak-item {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  padding: 0.75rem 0.9rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.weak-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.weak-cat {
  font-weight: 700;
  font-size: 0.85rem;
  color: var(--text-heading);
}

.weak-advise {
  font-size: 0.75rem;
  color: var(--text-muted);
  line-height: 1.35;
}

/* Recommendations */
.recommendations-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
}

.rec-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 1rem;
}

.rec-box {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 1rem 1.15rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.rec-top {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.rec-icon {
  font-size: 1.2rem;
}

.rec-title {
  color: var(--text-heading);
  font-size: 0.88rem;
}

.rec-body {
  font-size: 0.78rem;
  color: var(--text-muted);
  line-height: 1.4;
}
</style>
