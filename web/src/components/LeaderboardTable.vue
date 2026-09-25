<template>
  <div class="leaderboard-wrapper">
    <div class="table-toolbar">
      <div class="toolbar-left">
        <div class="search-box">
          <input
            v-model="searchQuery"
            type="text"
            placeholder="🔍 搜索模型名称或标识..."
            class="table-search-input"
          />
        </div>
        <div class="filter-chips">
          <button
            v-for="chip in filterChips"
            :key="chip.id"
            class="chip-btn"
            :class="{ active: activeFilterChip === chip.id }"
            @click="activeFilterChip = chip.id"
          >
            {{ chip.label }}
          </button>
        </div>
      </div>
      <div class="actions">
        <button class="btn btn-secondary" @click="emitExport">
          📥 导出排行榜 CSV
        </button>
      </div>
    </div>

    <div class="table-container">
      <table class="data-table">
        <thead>
          <tr>
            <th @click="sortBy('rank')" class="sortable">排名 <span class="sort-icon">{{ sortIcon('rank') }}</span></th>
            <th @click="sortBy('model_name')" class="sortable">评测模型 <span class="sort-icon">{{ sortIcon('model_name') }}</span></th>
            <th @click="sortBy('elo')" class="sortable">天梯 Elo <span class="sort-icon">{{ sortIcon('elo') }}</span></th>
            <th @click="sortBy('overall_accuracy')" class="sortable">微观准确率 (Micro) <span class="sort-icon">{{ sortIcon('overall_accuracy') }}</span></th>
            <th @click="sortBy('macro_accuracy')" class="sortable">宏观准确率 (Macro) <span class="sort-icon">{{ sortIcon('macro_accuracy') }}</span></th>
            <th @click="sortBy('l4_l5_frontier_accuracy')" class="sortable">L4/L5 极限难题 <span class="sort-icon">{{ sortIcon('l4_l5_frontier_accuracy') }}</span></th>
            <th @click="sortBy('weighted_composite_index')" class="sortable">综合指数 <span class="sort-icon">{{ sortIcon('weighted_composite_index') }}</span></th>
            <th @click="sortBy('avg_latency_ms')" class="sortable">平均延迟 <span class="sort-icon">{{ sortIcon('avg_latency_ms') }}</span></th>
            <th @click="sortBy('avg_ttft_ms')" class="sortable">首字延迟 (TTFT) <span class="sort-icon">{{ sortIcon('avg_ttft_ms') }}</span></th>
            <th @click="sortBy('avg_tps')" class="sortable">吞吐速率 (TPS) <span class="sort-icon">{{ sortIcon('avg_tps') }}</span></th>
            <th @click="sortBy('total_cost_usd')" class="sortable">总成本 ($) <span class="sort-icon">{{ sortIcon('total_cost_usd') }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(model, index) in sortedModels"
            :key="model.model_id"
            :class="{ selected: selectedModelId === model.model_id }"
            @click="$emit('select-model', model)"
          >
            <td>
              <span class="rank-badge" :class="'rank-' + (index + 1)">
                {{ index === 0 ? '🥇 1' : index === 1 ? '🥈 2' : index === 2 ? '🥉 3' : (index + 1) }}
              </span>
            </td>
            <td>
              <div class="model-info">
                <span class="model-title">{{ model.model_name }}</span>
                <span class="model-id-tag">{{ model.model_id }}</span>
              </div>
            </td>
            <td>
              <span class="elo-badge">{{ Math.round(model.computed_elo || model.elo_rating || 1200) }}</span>
            </td>
            <td>
              <div class="metric-progress">
                <div class="progress-bar-bg">
                  <div
                    class="progress-bar-fill"
                    :style="{
                      width: (model.overall_accuracy * 100) + '%',
                      background: getAccColor(model.overall_accuracy)
                    }"
                  ></div>
                </div>
                <span class="metric-val" :style="{ color: getAccColor(model.overall_accuracy) }">
                  {{ (model.overall_accuracy * 100).toFixed(1) }}%
                </span>
                <span class="cases-count">({{ model.passed_cases }}/{{ model.total_cases }})</span>
              </div>
            </td>
            <td>
              <span class="metric-pill" :style="{ color: getAccColor(model.macro_accuracy) }">
                {{ (model.macro_accuracy * 100).toFixed(1) }}%
              </span>
            </td>
            <td>
              <span class="badge" :class="getFrontierBadgeClass(model.l4_l5_frontier_accuracy)">
                ⚡ {{ (model.l4_l5_frontier_accuracy * 100).toFixed(1) }}%
              </span>
            </td>
            <td>
              <span class="composite-score">
                {{ (model.weighted_composite_index || model.overall_score * 100).toFixed(1) }}
              </span>
            </td>
            <td>
              <span class="latency-cell">{{ Math.round(model.avg_latency_ms) }} ms</span>
            </td>
            <td>
              <span class="ttft-cell">{{ model.avg_ttft_ms ? Math.round(model.avg_ttft_ms) + ' ms' : '-' }}</span>
            </td>
            <td>
              <span class="tps-cell">{{ model.avg_tps > 0 ? model.avg_tps.toFixed(1) + ' tok/s' : '-' }}</span>
            </td>
            <td>
              <span class="cost-cell">${{ model.total_cost_usd.toFixed(4) }}</span>
            </td>
          </tr>
          <tr v-if="sortedModels.length === 0">
            <td colspan="11" class="empty-cell">未找到匹配的模型数据</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue';
import { exportLeaderboardCsv } from '../utils/export';

const props = defineProps({
  models: {
    type: Array,
    required: true,
  },
  selectedModelId: {
    type: String,
    default: '',
  },
});

const emit = defineEmits(['select-model']);

const searchQuery = ref('');
const activeFilterChip = ref('all');
const sortKey = ref('overall_accuracy');
const sortOrder = ref('desc');

const filterChips = [
  { id: 'all', label: '全部模型' },
  { id: 'elo_top', label: '🏆 Elo 高分 (≥1200)' },
  { id: 'frontier_top', label: '⚡ 前沿卓越 (L4/L5≥60%)' },
  { id: 'low_latency', label: '🚀 极速低延 (≤2500ms)' },
  { id: 'cost_effective', label: '💰 经济高效 (≤$0.015)' },
];

function sortBy(key) {
  if (sortKey.value === key) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc';
  } else {
    sortKey.value = key;
    sortOrder.value = (key === 'rank' || key === 'model_name' || key === 'avg_latency_ms' || key === 'avg_ttft_ms' || key === 'total_cost_usd') ? 'asc' : 'desc';
  }
}

function sortIcon(key) {
  if (sortKey.value !== key) return '⇅';
  return sortOrder.value === 'asc' ? '▲' : '▼';
}

const sortedModels = computed(() => {
  let list = [...props.models];

  // Quick chips filter
  if (activeFilterChip.value === 'elo_top') {
    list = list.filter(m => (m.computed_elo || m.elo_rating || 1200) >= 1200);
  } else if (activeFilterChip.value === 'frontier_top') {
    list = list.filter(m => (m.l4_l5_frontier_accuracy || 0) >= 0.6);
  } else if (activeFilterChip.value === 'low_latency') {
    list = list.filter(m => (m.avg_latency_ms || 9999) <= 2500);
  } else if (activeFilterChip.value === 'cost_effective') {
    list = list.filter(m => (m.total_cost_usd || 0) <= 0.015);
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase().trim();
    list = list.filter(m => m.model_name.toLowerCase().includes(q) || m.model_id.toLowerCase().includes(q));
  }

  list.sort((a, b) => {
    let valA = a[sortKey.value];
    let valB = b[sortKey.value];

    if (sortKey.value === 'elo') {
      valA = a.computed_elo || a.elo_rating || 1200;
      valB = b.computed_elo || b.elo_rating || 1200;
    } else if (sortKey.value === 'rank') {
      valA = b.overall_accuracy;
      valB = a.overall_accuracy;
    }

    if (valA === undefined || valA === null) valA = -999999;
    if (valB === undefined || valB === null) valB = -999999;

    if (valA < valB) return sortOrder.value === 'asc' ? -1 : 1;
    if (valA > valB) return sortOrder.value === 'asc' ? 1 : -1;
    return 0;
  });

  return list;
});

function getAccColor(ratio) {
  const pct = (ratio || 0) * 100;
  if (pct >= 85) return 'var(--accent-emerald)';
  if (pct >= 60) return 'var(--accent-amber)';
  return 'var(--accent-rose)';
}

function getFrontierBadgeClass(ratio) {
  const pct = (ratio || 0) * 100;
  if (pct >= 85) return 'badge-success';
  if (pct >= 60) return 'badge-warning';
  return 'badge-danger';
}

function emitExport() {
  exportLeaderboardCsv(sortedModels.value);
}
</script>

<style scoped>
.leaderboard-wrapper {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: var(--shadow-card);
}

.table-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.4rem;
  border-bottom: 1px solid var(--border-soft);
  gap: 1rem;
  flex-wrap: wrap;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  flex-wrap: wrap;
}

.table-search-input {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  padding: 0.45rem 0.85rem;
  color: var(--text-heading);
  font-size: 0.84rem;
  width: 240px;
  outline: none;
  transition: all 0.2s;
}
.table-search-input:focus {
  border-color: var(--border-focus);
  box-shadow: 0 0 10px rgba(99, 102, 241, 0.2);
}

.filter-chips {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.chip-btn {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  color: var(--text-muted);
  font-size: 0.76rem;
  font-weight: 600;
  padding: 0.35rem 0.65rem;
  border-radius: 20px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.chip-btn:hover {
  background: var(--bg-hover);
  color: var(--text-heading);
  border-color: var(--border-strong);
}
.chip-btn.active {
  background: rgba(99, 102, 241, 0.15);
  border-color: var(--primary);
  color: var(--primary);
  font-weight: 700;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 1rem;
  border-radius: 8px;
  font-size: 0.84rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  border: 1px solid var(--border-soft);
}
.btn-secondary {
  background: var(--bg-subtle);
  color: var(--text-heading);
}
.btn-secondary:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

.table-container {
  overflow-x: auto;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: 0.88rem;
}

.data-table th {
  background: var(--bg-subtle);
  padding: 0.85rem 1.1rem;
  font-weight: 700;
  color: var(--text-muted);
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  border-bottom: 1px solid var(--border-strong);
  white-space: nowrap;
}

.data-table th.sortable {
  cursor: pointer;
  user-select: none;
}
.data-table th.sortable:hover {
  color: var(--text-heading);
}
.sort-icon {
  font-size: 0.75rem;
  margin-left: 0.2rem;
  opacity: 0.6;
}

.data-table tbody tr {
  border-bottom: 1px solid var(--border-soft);
  transition: all 0.15s ease;
  cursor: pointer;
}
.data-table tbody tr:hover {
  background: var(--bg-hover);
}
.data-table tbody tr.selected {
  background: rgba(99, 102, 241, 0.12);
  border-left: 3px solid var(--primary);
}

.data-table td {
  padding: 0.95rem 1.1rem;
  white-space: nowrap;
}

.model-info {
  display: flex;
  flex-direction: column;
}
.model-title {
  font-weight: 700;
  color: var(--text-heading);
}
.model-id-tag {
  font-size: 0.74rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
}

.elo-badge {
  font-family: var(--font-mono);
  font-weight: 800;
  color: #c084fc;
}

.metric-progress {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 170px;
}
.progress-bar-bg {
  flex: 1;
  height: 6px;
  background: var(--bg-subtle);
  border-radius: 4px;
  overflow: hidden;
}
.progress-bar-fill {
  height: 100%;
  border-radius: 4px;
}
.metric-val {
  font-weight: 700;
  font-size: 0.85rem;
}
.cases-count {
  font-size: 0.74rem;
  color: var(--text-faint);
}

.composite-score {
  font-weight: 800;
  color: var(--accent-sky);
}

.latency-cell, .ttft-cell, .tps-cell, .cost-cell {
  font-family: var(--font-mono);
  font-size: 0.82rem;
}
.ttft-cell { color: var(--accent-emerald); }
.tps-cell { color: var(--accent-amber); font-weight: 600; }
.cost-cell { color: var(--accent-rose); }

.empty-cell {
  text-align: center;
  padding: 3rem;
  color: var(--text-faint);
}
</style>
