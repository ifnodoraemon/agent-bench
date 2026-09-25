<template>
  <div class="radar-analysis-container">
    <!-- Top Model Selection Controls -->
    <div class="analysis-header-card">
      <div class="header-left">
        <h3 class="panel-title">🕸️ 七大领域全能力雷达与细分分析</h3>
        <p class="panel-subtitle">对比各模型在基础模型力、垂直专业、自主智能体多轮协作与安全对齐维度的综合表现</p>
      </div>
      <div class="model-selector-group">
        <span class="selector-label">选择对比模型:</span>
        <button
          v-for="(model, idx) in models"
          :key="model.model_id"
          class="model-chip"
          :class="{ active: isSelected(model.model_id) }"
          :style="getChipStyle(model.model_id, idx)"
          @click="toggleModel(model.model_id)"
        >
          <span class="chip-color-dot" :style="{ backgroundColor: getModelColor(model.model_id, idx) }"></span>
          <span class="chip-text">{{ model.model_name || model.model_id }}</span>
        </button>
      </div>
    </div>

    <!-- Charts Grid: Left Radar, Right Category Bar -->
    <div class="charts-grid">
      <div class="chart-card radar-card">
        <div class="chart-header">
          <h4>七大核心领域综合雷达对比 (Pass Rate %)</h4>
          <span class="chart-hint">基于 7 大领域能力建模</span>
        </div>
        <div class="chart-canvas-wrapper">
          <canvas ref="radarCanvasRef"></canvas>
        </div>
      </div>

      <div class="chart-card bar-card">
        <div class="chart-header">
          <h4>18 个子类细分通过率对比</h4>
          <div class="bar-toggle">
            <button
              class="btn-toggle-sub"
              :class="{ active: barViewMode === 'accuracy' }"
              @click="barViewMode = 'accuracy'"
            >
              通过率 (%)
            </button>
            <button
              class="btn-toggle-sub"
              :class="{ active: barViewMode === 'tps' }"
              @click="barViewMode = 'tps'"
            >
              生成吞吐 (TPS)
            </button>
          </div>
        </div>
        <div class="chart-canvas-wrapper bar-wrapper">
          <canvas ref="barCanvasRef"></canvas>
        </div>
      </div>
    </div>

    <!-- Domain Champions Ribbon -->
    <div class="champions-grid">
      <div
        v-for="domain in DOMAIN_DEFINITIONS"
        :key="domain.key"
        class="domain-stat-card"
      >
        <div class="stat-top">
          <span class="domain-icon">{{ domain.icon }}</span>
          <div class="domain-info">
            <span class="domain-name">{{ domain.name }}</span>
            <span class="domain-desc">{{ domain.description }}</span>
          </div>
        </div>
        <div class="champion-row">
          <span class="champ-label">最佳表现:</span>
          <span class="champ-name">{{ getDomainLeader(domain.key).name }}</span>
          <span class="badge badge-success">{{ (getDomainLeader(domain.key).score * 100).toFixed(1) }}%</span>
        </div>
        <div class="domain-mini-bars">
          <div
            v-for="(model, idx) in activeModels"
            :key="model.model_id"
            class="mini-bar-row"
          >
            <span class="mini-model-name">{{ model.model_name || model.model_id }}:</span>
            <div class="mini-track">
              <div
                class="mini-fill"
                :style="{
                  width: ((model.domain_scores?.[domain.key] || 0) * 100) + '%',
                  backgroundColor: getModelColor(model.model_id, idx)
                }"
              ></div>
            </div>
            <span class="mini-val">{{ ((model.domain_scores?.[domain.key] || 0) * 100).toFixed(1) }}%</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch, nextTick, onBeforeUnmount } from 'vue';
import Chart from 'chart.js/auto';
import { DOMAIN_DEFINITIONS, CATEGORY_NAMES } from '../utils/benchmark';

const props = defineProps({
  models: {
    type: Array,
    required: true
  }
});

const radarCanvasRef = ref(null);
const barCanvasRef = ref(null);
let radarChartInstance = null;
let barChartInstance = null;

const barViewMode = ref('accuracy'); // 'accuracy' or 'tps'

// Color palette for models
const COLOR_PALETTE = [
  '#6366f1', // Indigo
  '#10b981', // Emerald
  '#f59e0b', // Amber
  '#0ea5e9', // Sky
  '#a855f7', // Purple
  '#f43f5e', // Rose
  '#14b8a6', // Teal
  '#ec4899'  // Pink
];

// Active selected models IDs
const selectedModelIds = ref([]);

// Initialize with up to 3 models selected
watch(() => props.models, (newModels) => {
  if (newModels.length > 0 && selectedModelIds.value.length === 0) {
    selectedModelIds.value = newModels.slice(0, 3).map(m => m.model_id);
  }
  updateCharts();
}, { immediate: true });

watch(barViewMode, () => {
  updateBarChart();
});

const activeModels = computed(() => {
  return props.models.filter(m => selectedModelIds.value.includes(m.model_id));
});

function isSelected(id) {
  return selectedModelIds.value.includes(id);
}

function toggleModel(id) {
  if (isSelected(id)) {
    if (selectedModelIds.value.length > 1) {
      selectedModelIds.value = selectedModelIds.value.filter(x => x !== id);
    }
  } else {
    selectedModelIds.value.push(id);
  }
  updateCharts();
}

function getModelIndex(id) {
  return props.models.findIndex(m => m.model_id === id);
}

function getModelColor(id, fallbackIdx = 0) {
  const idx = getModelIndex(id);
  return COLOR_PALETTE[(idx >= 0 ? idx : fallbackIdx) % COLOR_PALETTE.length];
}

function getChipStyle(id, idx) {
  const color = getModelColor(id, idx);
  if (isSelected(id)) {
    return {
      borderColor: color,
      backgroundColor: color + '22',
      color: '#fff'
    };
  }
  return {};
}

function getDomainLeader(domainKey) {
  if (!props.models || props.models.length === 0) return { name: 'N/A', score: 0 };
  let best = props.models[0];
  let bestScore = best.domain_scores?.[domainKey] || 0;
  for (const m of props.models) {
    const sc = m.domain_scores?.[domainKey] || 0;
    if (sc > bestScore) {
      bestScore = sc;
      best = m;
    }
  }
  return { name: best.model_name || best.model_id, score: bestScore };
}

function initRadarChart() {
  if (!radarCanvasRef.value) return;
  if (radarChartInstance) radarChartInstance.destroy();

  const labels = DOMAIN_DEFINITIONS.map(d => d.name);

  const datasets = activeModels.value.map((m, idx) => {
    const color = getModelColor(m.model_id, idx);
    const data = DOMAIN_DEFINITIONS.map(d => {
      const sc = (m.domain_scores?.[d.key] || 0) * 100;
      return Math.round(sc * 10) / 10;
    });

    return {
      label: m.model_name || m.model_id,
      data,
      borderColor: color,
      backgroundColor: color + '2a',
      borderWidth: 2.2,
      pointBackgroundColor: color,
      pointBorderColor: '#fff',
      pointHoverRadius: 6,
      pointRadius: 4
    };
  });

  radarChartInstance = new Chart(radarCanvasRef.value, {
    type: 'radar',
    data: { labels, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        r: {
          min: 0,
          max: 100,
          ticks: {
            stepSize: 20,
            color: '#64748b',
            backdropColor: 'transparent',
            font: { size: 10, family: 'monospace' }
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.08)'
          },
          angleLines: {
            color: 'rgba(255, 255, 255, 0.08)'
          },
          pointLabels: {
            color: '#cbd5e1',
            font: {
              size: 11,
              weight: 'bold'
            }
          }
        }
      },
      plugins: {
        legend: {
          position: 'top',
          labels: {
            color: '#cbd5e1',
            font: { size: 12, weight: '600' },
            boxWidth: 14,
            usePointStyle: true
          }
        },
        tooltip: {
          callbacks: {
            label: (ctx) => ` ${ctx.dataset.label}: ${ctx.raw}%`
          }
        }
      }
    }
  });
}

function updateBarChart() {
  if (!barCanvasRef.value) return;
  if (barChartInstance) barChartInstance.destroy();

  // Distinct category keys
  const categoryKeys = Object.keys(CATEGORY_NAMES);
  const labels = categoryKeys.map(k => CATEGORY_NAMES[k]);

  const datasets = activeModels.value.map((m, idx) => {
    const color = getModelColor(m.model_id, idx);
    const data = categoryKeys.map(catKey => {
      const summary = m.category_summaries?.[catKey];
      if (!summary) return 0;
      if (barViewMode.value === 'accuracy') {
        return Math.round((summary.accuracy || 0) * 1000) / 10;
      } else {
        return Math.round((summary.avg_tps || 0) * 10) / 10;
      }
    });

    return {
      label: m.model_name || m.model_id,
      data,
      backgroundColor: color + 'cc',
      borderColor: color,
      borderWidth: 1,
      borderRadius: 4
    };
  });

  barChartInstance = new Chart(barCanvasRef.value, {
    type: 'bar',
    data: { labels, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      indexAxis: 'y', // Horizontal bars
      scales: {
        x: {
          min: 0,
          max: barViewMode.value === 'accuracy' ? 100 : undefined,
          grid: { color: 'rgba(255, 255, 255, 0.06)' },
          ticks: {
            color: '#94a3b8',
            callback: (v) => barViewMode.value === 'accuracy' ? v + '%' : v + ' tps'
          }
        },
        y: {
          grid: { display: false },
          ticks: {
            color: '#cbd5e1',
            font: { size: 11, weight: '500' }
          }
        }
      },
      plugins: {
        legend: {
          position: 'top',
          labels: {
            color: '#cbd5e1',
            font: { size: 11 },
            boxWidth: 12,
            usePointStyle: true
          }
        },
        tooltip: {
          callbacks: {
            label: (ctx) => ` ${ctx.dataset.label}: ${ctx.raw}${barViewMode.value === 'accuracy' ? '%' : ' tok/s'}`
          }
        }
      }
    }
  });
}

function updateCharts() {
  nextTick(() => {
    initRadarChart();
    updateBarChart();
  });
}

onMounted(() => {
  updateCharts();
});

onBeforeUnmount(() => {
  if (radarChartInstance) radarChartInstance.destroy();
  if (barChartInstance) barChartInstance.destroy();
});
</script>

<style scoped>
.radar-analysis-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.analysis-header-card {
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
  margin-bottom: 0.25rem;
}

.panel-subtitle {
  color: var(--text-muted);
  font-size: 0.85rem;
}

.model-selector-group {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.selector-label {
  font-size: 0.85rem;
  color: var(--text-muted);
  font-weight: 600;
  margin-right: 0.25rem;
}

.model-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  background: var(--bg-subtle);
  border: 1px solid var(--border-soft);
  color: var(--text-body);
  padding: 0.35rem 0.75rem;
  border-radius: 20px;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.model-chip:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

.chip-color-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.charts-grid {
  display: grid;
  grid-template-columns: 1fr 1.2fr;
  gap: 1.5rem;
}

@media (max-width: 1024px) {
  .charts-grid {
    grid-template-columns: 1fr;
  }
}

.chart-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.chart-header h4 {
  color: var(--text-heading);
  font-size: 1rem;
  font-weight: 700;
}

.chart-hint {
  font-size: 0.75rem;
  color: var(--text-faint);
}

.bar-toggle {
  display: flex;
  gap: 0.25rem;
  background: var(--bg-surface);
  border-radius: 6px;
  padding: 0.15rem;
  border: 1px solid var(--border-soft);
}

.btn-toggle-sub {
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-toggle-sub.active {
  background: var(--primary);
  color: #fff;
}

.chart-canvas-wrapper {
  position: relative;
  height: 420px;
  width: 100%;
}

.bar-wrapper {
  height: 520px;
}

/* Champions Ribbon */
.champions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 1rem;
}

.domain-stat-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 1rem 1.15rem;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.domain-stat-card:hover {
  transform: translateY(-2px);
  border-color: var(--border-strong);
}

.stat-top {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
}

.domain-icon {
  font-size: 1.6rem;
  line-height: 1;
}

.domain-info {
  display: flex;
  flex-direction: column;
}

.domain-name {
  color: var(--text-heading);
  font-weight: 700;
  font-size: 0.95rem;
}

.domain-desc {
  color: var(--text-muted);
  font-size: 0.75rem;
  line-height: 1.3;
  margin-top: 0.15rem;
}

.champion-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.4rem 0.6rem;
  background: var(--bg-surface);
  border-radius: 6px;
  font-size: 0.8rem;
}

.champ-label {
  color: var(--text-faint);
}

.champ-name {
  font-weight: 700;
  color: var(--text-heading);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.domain-mini-bars {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  margin-top: 0.25rem;
}

.mini-bar-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
}

.mini-model-name {
  width: 90px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mini-track {
  flex: 1;
  height: 6px;
  background: var(--bg-subtle);
  border-radius: 3px;
  overflow: hidden;
}

.mini-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}

.mini-val {
  width: 42px;
  text-align: right;
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--text-heading);
}
</style>
