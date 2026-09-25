<template>
  <div class="model-comparison-container">
    <!-- Battle Selection & Header -->
    <div class="battle-header-card">
      <div class="model-pick-block">
        <label>模型 A (蓝色方):</label>
        <select v-model="modelAId" class="battle-select model-a-select">
          <option v-for="m in models" :key="m.model_id" :value="m.model_id">
            {{ m.model_name || m.model_id }}
          </option>
        </select>
      </div>

      <button class="btn-swap-models" @click="swapModels" title="对调双方位置">
        ⇄ 对调
      </button>

      <div class="model-pick-block">
        <label>模型 B (橙红色方):</label>
        <select v-model="modelBId" class="battle-select model-b-select">
          <option v-for="m in models" :key="m.model_id" :value="m.model_id">
            {{ m.model_name || m.model_id }}
          </option>
        </select>
      </div>
    </div>

    <!-- Battle Scoreboard -->
    <div v-if="battleStats" class="battle-scoreboard-card">
      <div class="scoreboard-main">
        <div class="side-stat side-a">
          <span class="side-name">{{ modelA?.model_name || modelAId }}</span>
          <span class="side-score">{{ battleStats.winsA }} 胜</span>
          <span class="side-percent">{{ ((battleStats.winsA / (battleStats.total || 1)) * 100).toFixed(1) }}% 胜率</span>
        </div>

        <div class="center-tie-stat">
          <span class="tie-title">平局 / 同分</span>
          <span class="tie-count">{{ battleStats.ties }} 次</span>
          <span class="total-battles">共 {{ battleStats.total }} 题同场对决</span>
        </div>

        <div class="side-stat side-b">
          <span class="side-name">{{ modelB?.model_name || modelBId }}</span>
          <span class="side-score">{{ battleStats.winsB }} 胜</span>
          <span class="side-percent">{{ ((battleStats.winsB / (battleStats.total || 1)) * 100).toFixed(1) }}% 胜率</span>
        </div>
      </div>

      <!-- Segmented Battle Bar -->
      <div class="battle-bar">
        <div
          class="bar-seg seg-a"
          :style="{ width: ((battleStats.winsA / (battleStats.total || 1)) * 100) + '%' }"
          :title="`模型 A 获胜: ${battleStats.winsA}`"
        ></div>
        <div
          class="bar-seg seg-tie"
          :style="{ width: ((battleStats.ties / (battleStats.total || 1)) * 100) + '%' }"
          :title="`平局: ${battleStats.ties}`"
        ></div>
        <div
          class="bar-seg seg-b"
          :style="{ width: ((battleStats.winsB / (battleStats.total || 1)) * 100) + '%' }"
          :title="`模型 B 获胜: ${battleStats.winsB}`"
        ></div>
      </div>
    </div>

    <!-- Metrics Side-by-Side Table -->
    <div class="metrics-battle-card">
      <div class="card-header">
        <h4>全维度指标对决雷达对比</h4>
      </div>
      <table class="data-table battle-table">
        <thead>
          <tr>
            <th style="width: 32%;">{{ modelA?.model_name || modelAId }}</th>
            <th style="width: 36%; text-align: center;">评测核心指标</th>
            <th style="width: 32%; text-align: right;">{{ modelB?.model_name || modelBId }}</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('elo', true) }">
                {{ Math.round(modelA?.computed_elo || modelA?.elo_rating || 1200) }}
              </span>
            </td>
            <td class="metric-lbl">天梯 Elo 分值</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('elo', false) }">
                {{ Math.round(modelB?.computed_elo || modelB?.elo_rating || 1200) }}
              </span>
            </td>
          </tr>

          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('overall_accuracy', true) }">
                {{ ((modelA?.overall_accuracy || 0) * 100).toFixed(1) }}%
              </span>
            </td>
            <td class="metric-lbl">微观准确率 (Micro)</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('overall_accuracy', false) }">
                {{ ((modelB?.overall_accuracy || 0) * 100).toFixed(1) }}%
              </span>
            </td>
          </tr>

          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('macro_accuracy', true) }">
                {{ ((modelA?.macro_accuracy || 0) * 100).toFixed(1) }}%
              </span>
            </td>
            <td class="metric-lbl">宏观平衡准确率 (Macro)</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('macro_accuracy', false) }">
                {{ ((modelB?.macro_accuracy || 0) * 100).toFixed(1) }}%
              </span>
            </td>
          </tr>

          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('frontier', true) }">
                {{ ((modelA?.l4_l5_frontier_accuracy || 0) * 100).toFixed(1) }}%
              </span>
            </td>
            <td class="metric-lbl">⚡ L4/L5 极限前沿通过率</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('frontier', false) }">
                {{ ((modelB?.l4_l5_frontier_accuracy || 0) * 100).toFixed(1) }}%
              </span>
            </td>
          </tr>

          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('latency', true, true) }">
                {{ Math.round(modelA?.avg_latency_ms || 0) }} ms
              </span>
            </td>
            <td class="metric-lbl">端到端平均延迟 (越低越好)</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('latency', false, true) }">
                {{ Math.round(modelB?.avg_latency_ms || 0) }} ms
              </span>
            </td>
          </tr>

          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('ttft', true, true) }">
                {{ modelA?.avg_ttft_ms ? Math.round(modelA.avg_ttft_ms) + ' ms' : '-' }}
              </span>
            </td>
            <td class="metric-lbl">首字到达延迟 TTFT (越低越好)</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('ttft', false, true) }">
                {{ modelB?.avg_ttft_ms ? Math.round(modelB.avg_ttft_ms) + ' ms' : '-' }}
              </span>
            </td>
          </tr>

          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('tps', true) }">
                {{ modelA?.avg_tps > 0 ? modelA.avg_tps.toFixed(1) + ' tok/s' : '-' }}
              </span>
            </td>
            <td class="metric-lbl">推理吞吐 TPS (越高越好)</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('tps', false) }">
                {{ modelB?.avg_tps > 0 ? modelB.avg_tps.toFixed(1) + ' tok/s' : '-' }}
              </span>
            </td>
          </tr>

          <tr>
            <td>
              <span class="metric-val" :class="{ winner: isWinner('cost', true, true) }">
                ${{ (modelA?.total_cost_usd || 0).toFixed(4) }}
              </span>
            </td>
            <td class="metric-lbl">评测总耗费 USD (越低越好)</td>
            <td style="text-align: right;">
              <span class="metric-val" :class="{ winner: isWinner('cost', false, true) }">
                ${{ (modelB?.total_cost_usd || 0).toFixed(4) }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Case Discrepancy Explorer -->
    <div class="discrepancy-card">
      <div class="disc-header">
        <div class="disc-title-block">
          <h4>两模型用例同场差异探测</h4>
          <span class="chart-hint">筛选模型表现分歧的题目，排查模型特定短板</span>
        </div>

        <div class="disc-filter-group">
          <button
            class="disc-filter-btn"
            :class="{ active: discFilter === 'winA' }"
            @click="discFilter = 'winA'"
          >
            仅看 A 胜 B 败 ({{ winACases.length }})
          </button>
          <button
            class="disc-filter-btn"
            :class="{ active: discFilter === 'winB' }"
            @click="discFilter = 'winB'"
          >
            仅看 B 胜 A 败 ({{ winBCases.length }})
          </button>
          <button
            class="disc-filter-btn"
            :class="{ active: discFilter === 'both_fail' }"
            @click="discFilter = 'both_fail'"
          >
            两者均失败 ({{ bothFailCases.length }})
          </button>
          <button
            class="disc-filter-btn"
            :class="{ active: discFilter === 'all' }"
            @click="discFilter = 'all'"
          >
            全部同场题 ({{ (battleStats?.commonCases || []).length }})
          </button>
        </div>
      </div>

      <div class="disc-cases-list">
        <div v-if="filteredDiscrepancyCases.length === 0" class="empty-notice">
          该分类下无差异用例
        </div>

        <div
          v-for="item in pagedDiscrepancyCases"
          :key="item.id"
          class="diff-case-item"
        >
          <div class="diff-header" @click="toggleDiffExpand(item.id)">
            <div class="diff-title-row">
              <span class="case-id">{{ item.id }}</span>
              <span class="badge" :class="'badge-tier-' + (item.difficulty || 'l3').toLowerCase()">
                {{ item.difficulty || 'L3' }}
              </span>
              <span class="badge badge-info">{{ CATEGORY_NAMES[item.category] || item.category }}</span>
              <span class="outcome-badge" :class="'badge-' + item.outcome">
                {{ getOutcomeText(item.outcome) }}
              </span>
            </div>
            <button class="expand-text-btn">
              {{ isDiffExpanded(item.id) ? '▲ 收起对比' : '▼ 展开双模型响应' }}
            </button>
          </div>

          <!-- Side-by-side expanded output -->
          <div v-if="isDiffExpanded(item.id)" class="diff-side-by-side">
            <div class="diff-column col-a">
              <div class="col-header">
                <strong>{{ modelA?.model_name || modelAId }}</strong>
                <span class="badge" :class="item.caseA?.passed ? 'badge-success' : 'badge-danger'">
                  {{ item.caseA?.passed ? '✅ 满分通过' : '❌ 失败' }} ({{ (item.caseA?.score || 0).toFixed(2) }})
                </span>
              </div>
              <div v-if="getThinking(item.caseA).thinking" class="code-box thinking-box-sub">
                <span class="box-lbl">🧠 思考链:</span>
                {{ getThinking(item.caseA).thinking }}
              </div>
              <div class="code-box diff-output-box">
                <span class="box-lbl">💬 输出:</span>
                {{ getThinking(item.caseA).response || '（无响应）' }}
              </div>
              <div class="code-box diff-reason-box">
                <span class="box-lbl">🧪 验证:</span>
                {{ item.caseA?.reason || '（无测试反馈）' }}
              </div>
            </div>

            <div class="diff-column col-b">
              <div class="col-header">
                <strong>{{ modelB?.model_name || modelBId }}</strong>
                <span class="badge" :class="item.caseB?.passed ? 'badge-success' : 'badge-danger'">
                  {{ item.caseB?.passed ? '✅ 满分通过' : '❌ 失败' }} ({{ (item.caseB?.score || 0).toFixed(2) }})
                </span>
              </div>
              <div v-if="getThinking(item.caseB).thinking" class="code-box thinking-box-sub">
                <span class="box-lbl">🧠 思考链:</span>
                {{ getThinking(item.caseB).thinking }}
              </div>
              <div class="code-box diff-output-box">
                <span class="box-lbl">💬 输出:</span>
                {{ getThinking(item.caseB).response || '（无响应）' }}
              </div>
              <div class="code-box diff-reason-box">
                <span class="box-lbl">🧪 验证:</span>
                {{ item.caseB?.reason || '（无测试反馈）' }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue';
import { compareTwoModels } from '../utils/elo';
import { CATEGORY_NAMES, extractThinking } from '../utils/benchmark';

const props = defineProps({
  models: {
    type: Array,
    required: true
  }
});

const modelAId = ref('');
const modelBId = ref('');

watch(() => props.models, (newModels) => {
  if (newModels.length >= 2) {
    if (!modelAId.value) modelAId.value = newModels[0].model_id;
    if (!modelBId.value || modelBId.value === modelAId.value) {
      modelBId.value = newModels[1].model_id;
    }
  } else if (newModels.length === 1) {
    modelAId.value = newModels[0].model_id;
    modelBId.value = newModels[0].model_id;
  }
}, { immediate: true });

function swapModels() {
  const tmp = modelAId.value;
  modelAId.value = modelBId.value;
  modelBId.value = tmp;
}

const modelA = computed(() => props.models.find(m => m.model_id === modelAId.value));
const modelB = computed(() => props.models.find(m => m.model_id === modelBId.value));

const battleStats = computed(() => {
  if (!modelA.value || !modelB.value) return null;
  return compareTwoModels(modelA.value, modelB.value);
});

function isWinner(metric, isA, lowerIsBetter = false) {
  if (!modelA.value || !modelB.value) return false;
  let valA = 0;
  let valB = 0;

  switch (metric) {
    case 'elo':
      valA = modelA.value.computed_elo || modelA.value.elo_rating || 1200;
      valB = modelB.value.computed_elo || modelB.value.elo_rating || 1200;
      break;
    case 'overall_accuracy':
      valA = modelA.value.overall_accuracy || 0;
      valB = modelB.value.overall_accuracy || 0;
      break;
    case 'macro_accuracy':
      valA = modelA.value.macro_accuracy || 0;
      valB = modelB.value.macro_accuracy || 0;
      break;
    case 'frontier':
      valA = modelA.value.l4_l5_frontier_accuracy || 0;
      valB = modelB.value.l4_l5_frontier_accuracy || 0;
      break;
    case 'latency':
      valA = modelA.value.avg_latency_ms || 0;
      valB = modelB.value.avg_latency_ms || 0;
      break;
    case 'ttft':
      valA = modelA.value.avg_ttft_ms || 0;
      valB = modelB.value.avg_ttft_ms || 0;
      break;
    case 'tps':
      valA = modelA.value.avg_tps || 0;
      valB = modelB.value.avg_tps || 0;
      break;
    case 'cost':
      valA = modelA.value.total_cost_usd || 0;
      valB = modelB.value.total_cost_usd || 0;
      break;
  }

  if (lowerIsBetter) {
    if (valA === 0 || valB === 0) return false;
    return isA ? valA < valB : valB < valA;
  }
  return isA ? valA > valB : valB > valA;
}

// Discrepancy filters
const discFilter = ref('winA'); // 'winA', 'winB', 'both_fail', 'all'
const expandedDiffs = ref(new Set());

function isDiffExpanded(id) {
  return expandedDiffs.value.has(id);
}

function toggleDiffExpand(id) {
  if (expandedDiffs.value.has(id)) {
    expandedDiffs.value.delete(id);
  } else {
    expandedDiffs.value.add(id);
  }
}

const allBattleCases = computed(() => {
  return battleStats.value?.commonCases || [];
});

const winACases = computed(() => {
  return allBattleCases.value.filter(c => c.outcome === 'winA');
});

const winBCases = computed(() => {
  return allBattleCases.value.filter(c => c.outcome === 'winB');
});

const bothFailCases = computed(() => {
  return allBattleCases.value.filter(c => !c.caseA.passed && !c.caseB.passed);
});

const filteredDiscrepancyCases = computed(() => {
  if (discFilter.value === 'winA') return winACases.value;
  if (discFilter.value === 'winB') return winBCases.value;
  if (discFilter.value === 'both_fail') return bothFailCases.value;
  return allBattleCases.value;
});

const pagedDiscrepancyCases = computed(() => {
  // Show top 30 cases to keep DOM light and smooth
  return filteredDiscrepancyCases.value.slice(0, 30);
});

function getOutcomeText(outcome) {
  if (outcome === 'winA') return 'A 获胜';
  if (outcome === 'winB') return 'B 获胜';
  return '平局';
}

function getThinking(c) {
  if (!c) return { thinking: '', response: '' };
  return extractThinking(c.model_output);
}
</script>

<style scoped>
.model-comparison-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.battle-header-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem 1.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1.5rem;
  flex-wrap: wrap;
}

.model-pick-block {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.model-pick-block label {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--text-muted);
}

.battle-select {
  background: var(--bg-surface);
  border: 2px solid var(--border-soft);
  color: var(--text-heading);
  padding: 0.5rem 1rem;
  border-radius: 8px;
  font-size: 0.95rem;
  font-weight: 700;
  outline: none;
  min-width: 220px;
}

.model-a-select {
  border-color: rgba(99, 102, 241, 0.4);
}

.model-b-select {
  border-color: rgba(244, 63, 94, 0.4);
}

.btn-swap-models {
  background: var(--bg-subtle);
  border: 1px solid var(--border-soft);
  color: var(--text-heading);
  padding: 0.5rem 0.85rem;
  border-radius: 8px;
  font-weight: 700;
  cursor: pointer;
  align-self: flex-end;
  margin-bottom: 2px;
  transition: all 0.2s ease;
}

.btn-swap-models:hover {
  background: var(--bg-hover);
  border-color: var(--primary);
}

/* Scoreboard Card */
.battle-scoreboard-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.scoreboard-main {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.side-stat {
  display: flex;
  flex-direction: column;
}

.side-a { align-items: flex-start; }
.side-b { align-items: flex-end; }

.side-name {
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--text-muted);
}

.side-score {
  font-family: var(--font-mono);
  font-size: 2rem;
  font-weight: 900;
}

.side-a .side-score { color: #818cf8; }
.side-b .side-score { color: #fb7185; }

.side-percent {
  font-size: 0.8rem;
  color: var(--text-faint);
}

.center-tie-stat {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.tie-title {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.tie-count {
  font-family: var(--font-mono);
  font-size: 1.4rem;
  font-weight: 800;
  color: var(--text-heading);
}

.total-battles {
  font-size: 0.75rem;
  color: var(--text-faint);
}

.battle-bar {
  height: 12px;
  background: var(--bg-surface);
  border-radius: 6px;
  display: flex;
  overflow: hidden;
  border: 1px solid var(--border-soft);
}

.seg-a { background: #6366f1; transition: width 0.3s ease; }
.seg-tie { background: var(--bg-subtle); transition: width 0.3s ease; }
.seg-b { background: #f43f5e; transition: width 0.3s ease; }

/* Metrics Table */
.metrics-battle-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
}

.card-header {
  margin-bottom: 1rem;
}

.card-header h4 {
  color: var(--text-heading);
  font-size: 1.05rem;
  font-weight: 700;
}

.battle-table th {
  background: var(--bg-surface);
  color: var(--text-muted);
  font-size: 0.85rem;
  padding: 0.75rem 1rem;
}

.battle-table td {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--border-soft);
}

.metric-val {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 0.95rem;
  color: var(--text-body);
}

.metric-val.winner {
  color: var(--accent-emerald);
  font-weight: 900;
}

.metric-val.winner::after {
  content: '  👑';
  font-size: 0.8rem;
}

.metric-lbl {
  text-align: center;
  font-weight: 600;
  color: var(--text-muted);
  font-size: 0.82rem;
}

/* Discrepancy Card */
.discrepancy-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.disc-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
}

.disc-title-block h4 {
  color: var(--text-heading);
  font-size: 1.05rem;
  font-weight: 700;
}

.chart-hint {
  font-size: 0.8rem;
  color: var(--text-muted);
}

.disc-filter-group {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  padding: 0.2rem;
}

.disc-filter-btn {
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: 0.78rem;
  font-weight: 600;
  padding: 0.35rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.disc-filter-btn.active {
  background: var(--primary);
  color: #fff;
}

.disc-cases-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.empty-notice {
  text-align: center;
  padding: 2rem;
  color: var(--text-muted);
  font-size: 0.85rem;
}

.diff-case-item {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  overflow: hidden;
}

.diff-header {
  padding: 0.75rem 1rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
}

.diff-title-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.outcome-badge {
  font-size: 0.75rem;
  font-weight: 700;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
}

.badge-winA { background: rgba(99, 102, 241, 0.2); color: #818cf8; }
.badge-winB { background: rgba(244, 63, 94, 0.2); color: #fb7185; }
.badge-tie { background: var(--bg-subtle); color: var(--text-muted); }

.expand-text-btn {
  background: transparent;
  border: none;
  color: var(--primary);
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
}

.diff-side-by-side {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
  padding: 1rem;
  background: var(--bg-card);
  border-top: 1px solid var(--border-soft);
}

@media (max-width: 900px) {
  .diff-side-by-side {
    grid-template-columns: 1fr;
  }
}

.diff-column {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.col-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 0.4rem;
  border-bottom: 1px solid var(--border-soft);
}

.col-a .col-header strong { color: #818cf8; }
.col-b .col-header strong { color: #fb7185; }

.box-lbl {
  display: block;
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--text-faint);
  margin-bottom: 0.25rem;
}

.thinking-box-sub {
  border-color: rgba(168, 85, 247, 0.25);
  background: rgba(22, 16, 38, 0.5);
  color: #d8b4fe;
  max-height: 180px;
}

.diff-output-box {
  max-height: 220px;
}

.diff-reason-box {
  max-height: 150px;
  background: rgba(15, 23, 42, 0.6);
}
</style>
