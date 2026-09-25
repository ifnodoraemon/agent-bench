<template>
  <div class="pareto-analysis-container">
    <!-- Header Banner -->
    <div class="pareto-header-card">
      <div class="header-left">
        <h3 class="panel-title">📈 帕累托最优前沿与能效分析 (Pareto Frontier)</h3>
        <p class="panel-subtitle">
          在实际业务落地中，不仅追求极高准确率，更需权衡 <strong>延迟 (Latency)</strong>、<strong>吞吐 (TPS)</strong> 与 <strong>推理成本 (Cost)</strong>。
          帕累托边界标记了在既定资源约束下无法被其他模型在各维度同时超越的“不可替代”模型。
        </p>
      </div>

      <!-- Mode Selector -->
      <div class="mode-toggles">
        <button
          class="mode-btn"
          :class="{ active: chartMode === 'acc_latency' }"
          @click="chartMode = 'acc_latency'"
        >
          ⏱️ 准确率 vs 延迟
        </button>
        <button
          class="mode-btn"
          :class="{ active: chartMode === 'frontier_latency' }"
          @click="chartMode = 'frontier_latency'"
        >
          ⚡ L4/L5前沿 vs 延迟
        </button>
        <button
          class="mode-btn"
          :class="{ active: chartMode === 'acc_cost' }"
          @click="chartMode = 'acc_cost'"
        >
          💰 准确率 vs 成本 ($)
        </button>
      </div>
    </div>

    <!-- Champions Spotlight -->
    <div class="champions-spotlight-grid">
      <div class="spotlight-card speed-card">
        <div class="spotlight-icon">🚀</div>
        <div class="spotlight-body">
          <span class="spotlight-tag">极速推理先锋 (Speed King)</span>
          <span class="spotlight-name">{{ speedChampion?.model_name || '-' }}</span>
          <span class="spotlight-metric">延迟 {{ Math.round(speedChampion?.avg_latency_ms || 0) }} ms · {{ (speedChampion?.avg_tps || 0).toFixed(1) }} tok/s</span>
        </div>
      </div>

      <div class="spotlight-card intelligence-card">
        <div class="spotlight-icon">🧠</div>
        <div class="spotlight-body">
          <span class="spotlight-tag">极限智力巅峰 (Peak Frontier)</span>
          <span class="spotlight-name">{{ frontierChampion?.model_name || '-' }}</span>
          <span class="spotlight-metric">L4/L5 通过率 {{ ((frontierChampion?.l4_l5_frontier_accuracy || 0) * 100).toFixed(1) }}%</span>
        </div>
      </div>

      <div class="spotlight-card budget-card">
        <div class="spotlight-icon">💎</div>
        <div class="spotlight-body">
          <span class="spotlight-tag">综合能效最优 (Pareto Optimal)</span>
          <span class="spotlight-name">{{ paretoChampion?.model_name || '-' }}</span>
          <span class="spotlight-metric">微观 {{ ((paretoChampion?.overall_accuracy || 0) * 100).toFixed(1) }}% · ${{ (paretoChampion?.total_cost_usd || 0).toFixed(4) }}</span>
        </div>
      </div>
    </div>

    <!-- Scatter Chart -->
    <div class="chart-card">
      <div class="chart-header">
        <div class="header-titles">
          <h4>{{ chartTitle }}</h4>
          <span class="chart-hint">右上角方向代表更高智力与更低开销 (非支配集形成外沿边界)</span>
        </div>
      </div>
      <div class="scatter-canvas-wrapper">
        <canvas ref="scatterCanvasRef"></canvas>
      </div>
    </div>

    <!-- Pareto Metrics Matrix Table -->
    <div class="table-card">
      <div class="table-header">
        <h4>模型成本效能与抗压性指标矩阵</h4>
      </div>
      <table class="data-table">
        <thead>
          <tr>
            <th>评测模型</th>
            <th>帕累托前沿定位</th>
            <th>微观通过率</th>
            <th>L4/L5 极限前沿</th>
            <th>平均延迟</th>
            <th>推理 TPS</th>
            <th>千次测试预估成本</th>
            <th>前沿衰减韧性 (L5/L1)</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="m in models" :key="m.model_id">
            <td>
              <div class="model-meta-cell">
                <span class="m-title">{{ m.model_name || m.model_id }}</span>
                <span class="m-sub">{{ m.model_id }}</span>
              </div>
            </td>
            <td>
              <span class="badge" :class="isParetoOptimal(m) ? 'badge-purple' : 'badge-info'">
                {{ isParetoOptimal(m) ? '⭐ 帕累托最优边界' : '标准候选集' }}
              </span>
            </td>
            <td>
              <span class="metric-num">{{ ((m.overall_accuracy || 0) * 100).toFixed(1) }}%</span>
            </td>
            <td>
              <span class="metric-num text-purple">{{ ((m.l4_l5_frontier_accuracy || 0) * 100).toFixed(1) }}%</span>
            </td>
            <td>
              <span class="metric-num">{{ Math.round(m.avg_latency_ms || 0) }} ms</span>
            </td>
            <td>
              <span class="metric-num">{{ (m.avg_tps || 0).toFixed(1) }} tok/s</span>
            </td>
            <td>
              <span class="metric-num text-emerald">
                ${{ (calculateCostPer1k(m)).toFixed(3) }}
              </span>
            </td>
            <td>
              <div class="resilience-cell">
                <span class="resilience-val" :class="getResilienceClass(m)">
                  {{ getResiliencePercent(m) }}%
                </span>
                <span class="resilience-sub">
                  (L5: {{ ((m.tier_breakdown?.L5?.accuracy || 0) * 100).toFixed(0) }}% / L1: {{ ((m.tier_breakdown?.L1?.accuracy || 0) * 100).toFixed(0) }}%)
                </span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch, nextTick, onBeforeUnmount } from 'vue';
import Chart from 'chart.js/auto';

const props = defineProps({
  models: {
    type: Array,
    required: true
  }
});

const scatterCanvasRef = ref(null);
let scatterChartInstance = null;

const chartMode = ref('acc_latency'); // 'acc_latency' | 'frontier_latency' | 'acc_cost'

const COLOR_PALETTE = ['#6366f1', '#10b981', '#f59e0b', '#0ea5e9', '#a855f7', '#f43f5e'];

const chartTitle = computed(() => {
  if (chartMode.value === 'acc_latency') return '微观准确率 (Micro Acc %) vs 平均耗时 (ms)';
  if (chartMode.value === 'frontier_latency') return 'L4/L5 极限难题通过率 (%) vs 平均耗时 (ms)';
  return '微观准确率 (Micro Acc %) vs 评测总成本 (USD)';
});

const speedChampion = computed(() => {
  if (!props.models.length) return null;
  return [...props.models].filter(m => m.avg_latency_ms > 0).sort((a, b) => a.avg_latency_ms - b.avg_latency_ms)[0];
});

const frontierChampion = computed(() => {
  if (!props.models.length) return null;
  return [...props.models].sort((a, b) => b.l4_l5_frontier_accuracy - a.l4_l5_frontier_accuracy)[0];
});

const paretoChampion = computed(() => {
  if (!props.models.length) return null;
  return [...props.models].sort((a, b) => {
    const scoreA = (a.overall_accuracy * 100) / (Math.log(a.avg_latency_ms || 1000) * (a.total_cost_usd + 0.1));
    const scoreB = (b.overall_accuracy * 100) / (Math.log(b.avg_latency_ms || 1000) * (b.total_cost_usd + 0.1));
    return scoreB - scoreA;
  })[0];
});

function isParetoOptimal(m) {
  // A model is non-dominated if no other model has both strictly higher accuracy and strictly lower latency
  for (const other of props.models) {
    if (other.model_id === m.model_id) continue;
    if (other.overall_accuracy >= m.overall_accuracy && other.avg_latency_ms < m.avg_latency_ms) {
      return false;
    }
  }
  return true;
}

function calculateCostPer1k(m) {
  const totalCases = m.total_cases || m.case_results?.length || 1;
  return ((m.total_cost_usd || 0) / totalCases) * 1000;
}

function getResiliencePercent(m) {
  const l1 = m.tier_breakdown?.L1?.accuracy || 0.95;
  const l5 = m.tier_breakdown?.L5?.accuracy || 0;
  if (l1 <= 0) return 0;
  return ((l5 / l1) * 100).toFixed(1);
}

function getResilienceClass(m) {
  const val = parseFloat(getResiliencePercent(m));
  if (val >= 85) return 'resilience-high';
  if (val >= 75) return 'resilience-med';
  return 'resilience-low';
}

function updateScatterChart() {
  if (!scatterCanvasRef.value) return;
  if (scatterChartInstance) scatterChartInstance.destroy();

  const datasets = props.models.map((m, idx) => {
    const color = COLOR_PALETTE[idx % COLOR_PALETTE.length];
    let xVal = 0;
    let yVal = 0;

    if (chartMode.value === 'acc_latency') {
      xVal = Math.round(m.avg_latency_ms || 0);
      yVal = Math.round((m.overall_accuracy || 0) * 1000) / 10;
    } else if (chartMode.value === 'frontier_latency') {
      xVal = Math.round(m.avg_latency_ms || 0);
      yVal = Math.round((m.l4_l5_frontier_accuracy || 0) * 1000) / 10;
    } else {
      xVal = Math.round((m.total_cost_usd || 0) * 10000) / 10000;
      yVal = Math.round((m.overall_accuracy || 0) * 1000) / 10;
    }

    return {
      label: m.model_name || m.model_id,
      data: [{ x: xVal, y: yVal, model: m }],
      backgroundColor: color,
      borderColor: '#ffffff',
      borderWidth: 2,
      pointRadius: 10,
      pointHoverRadius: 13
    };
  });

  const xLabel = chartMode.value === 'acc_cost' ? '评测总成本 ($ USD)' : '平均延迟 (ms)';
  const yLabel = chartMode.value === 'frontier_latency' ? 'L4/L5 极限前沿通过率 (%)' : '微观准确率 (%)';

  scatterChartInstance = new Chart(scatterCanvasRef.value, {
    type: 'scatter',
    data: { datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        x: {
          title: {
            display: true,
            text: xLabel,
            color: '#94a3b8',
            font: { size: 12, weight: '600' }
          },
          grid: { color: 'rgba(255, 255, 255, 0.08)' },
          ticks: {
            color: '#94a3b8',
            font: { family: 'monospace' }
          }
        },
        y: {
          title: {
            display: true,
            text: yLabel,
            color: '#94a3b8',
            font: { size: 12, weight: '600' }
          },
          grid: { color: 'rgba(255, 255, 255, 0.08)' },
          ticks: {
            color: '#cbd5e1',
            font: { family: 'monospace' }
          }
        }
      },
      plugins: {
        legend: {
          position: 'top',
          labels: {
            color: '#cbd5e1',
            font: { size: 12, weight: '600' },
            boxWidth: 12,
            usePointStyle: true
          }
        },
        tooltip: {
          callbacks: {
            label: (ctx) => {
              const pt = ctx.raw;
              const unit = chartMode.value === 'acc_cost' ? '$' : 'ms';
              return ` ${ctx.dataset.label} — X: ${pt.x}${unit}, Y: ${pt.y}%`;
            }
          }
        }
      }
    }
  });
}

watch([() => props.models, chartMode], () => {
  nextTick(() => {
    updateScatterChart();
  });
}, { immediate: true });

onMounted(() => {
  updateScatterChart();
});

onBeforeUnmount(() => {
  if (scatterChartInstance) scatterChartInstance.destroy();
});
</script>

<style scoped>
.pareto-analysis-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.pareto-header-card {
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

.mode-toggles {
  display: flex;
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  padding: 0.2rem;
  gap: 0.25rem;
}

.mode-btn {
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: 0.8rem;
  font-weight: 600;
  padding: 0.4rem 0.85rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.mode-btn.active {
  background: var(--primary);
  color: #fff;
}

/* Champions Spotlight */
.champions-spotlight-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
}

.spotlight-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.15rem 1.35rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.spotlight-card:hover {
  transform: translateY(-2px);
  border-color: var(--border-strong);
}

.speed-card { border-left: 4px solid var(--accent-sky); }
.intelligence-card { border-left: 4px solid var(--accent-purple); }
.budget-card { border-left: 4px solid var(--accent-emerald); }

.spotlight-icon {
  font-size: 2rem;
  line-height: 1;
}

.spotlight-body {
  display: flex;
  flex-direction: column;
}

.spotlight-tag {
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--text-muted);
}

.spotlight-name {
  font-size: 1.1rem;
  font-weight: 800;
  color: var(--text-heading);
  margin: 0.15rem 0;
}

.spotlight-metric {
  font-size: 0.78rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
}

/* Chart */
.chart-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
}

.chart-header {
  margin-bottom: 1rem;
}

.chart-header h4 {
  color: var(--text-heading);
  font-size: 1.05rem;
  font-weight: 700;
}

.chart-hint {
  font-size: 0.8rem;
  color: var(--text-muted);
}

.scatter-canvas-wrapper {
  position: relative;
  height: 420px;
  width: 100%;
}

/* Table */
.table-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
}

.table-header {
  margin-bottom: 1rem;
}

.table-header h4 {
  color: var(--text-heading);
  font-size: 1.05rem;
  font-weight: 700;
}

.model-meta-cell {
  display: flex;
  flex-direction: column;
}

.m-title {
  font-weight: 700;
  color: var(--text-heading);
  font-size: 0.9rem;
}

.m-sub {
  font-size: 0.72rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
}

.metric-num {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 0.88rem;
  color: var(--text-body);
}

.text-purple { color: #c084fc; }
.text-emerald { color: #34d399; }

.resilience-cell {
  display: flex;
  flex-direction: column;
}

.resilience-val {
  font-family: var(--font-mono);
  font-weight: 800;
  font-size: 0.9rem;
}

.resilience-high { color: var(--accent-emerald); }
.resilience-med { color: var(--accent-amber); }
.resilience-low { color: var(--accent-rose); }

.resilience-sub {
  font-size: 0.7rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
}
</style>
