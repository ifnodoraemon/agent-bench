<template>
  <div class="tier-breakdown-container">
    <!-- Header Banner -->
    <div class="tier-header-card">
      <div class="header-left">
        <h3 class="panel-title">🪜 L1 ~ L5 五级难度阶梯分析</h3>
        <p class="panel-subtitle">
          传统基准易出现“得分虚高”，Agent-Bench 通过动态分级剔除无效方差：聚焦 <strong>L4 (自主智能体)</strong> 与 <strong>L5 (极限前沿难题)</strong> 的真实分水岭。
        </p>
      </div>
      <div class="frontier-badges">
        <div class="frontier-stat-pill">
          <span class="pill-label">前沿极限题量 (L4+L5)</span>
          <span class="pill-value">579 题</span>
        </div>
        <div class="frontier-stat-pill highlight">
          <span class="pill-label">高区分度题占比</span>
          <span class="pill-value">57.4%</span>
        </div>
      </div>
    </div>

    <!-- Tier Overview Cards -->
    <div class="tier-cards-grid">
      <div
        v-for="(info, tierKey) in TIER_DEFINITIONS"
        :key="tierKey"
        class="tier-info-card"
        :class="'card-' + tierKey.toLowerCase()"
      >
        <div class="tier-card-header">
          <span class="badge" :class="'badge-tier-' + tierKey.toLowerCase()">{{ tierKey }}</span>
          <span class="tier-level-title">{{ info.name }}</span>
        </div>
        <p class="tier-card-desc">{{ info.desc }}</p>
        <div class="tier-model-ranks">
          <div
            v-for="(model, idx) in models"
            :key="model.model_id"
            class="tier-model-score-row"
          >
            <span class="model-abbr">{{ model.model_name || model.model_id }}</span>
            <div class="score-track">
              <div
                class="score-fill"
                :style="{
                  width: ((model.tier_breakdown?.[tierKey]?.accuracy || 0) * 100) + '%',
                  backgroundColor: info.color
                }"
              ></div>
            </div>
            <span class="score-percent">
              {{ ((model.tier_breakdown?.[tierKey]?.accuracy || 0) * 100).toFixed(1) }}%
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Main Chart: Grouped Tier Comparison -->
    <div class="chart-section-card">
      <div class="chart-header">
        <div class="title-with-desc">
          <h4>各模型分级通过率全景图 (L1 ~ L5 Pass Rate %)</h4>
          <span class="chart-hint">清晰展现模型在 L4 工具编排与 L5 竞赛级难度下的性能断崖</span>
        </div>
      </div>
      <div class="chart-canvas-wrapper">
        <canvas ref="tierChartRef"></canvas>
      </div>
    </div>

    <!-- Detailed Tier Matrix Table -->
    <div class="tier-table-card">
      <div class="table-header">
        <h4>难度阶梯详尽性能矩阵</h4>
      </div>
      <div class="table-responsive">
        <table class="data-table tier-matrix-table">
          <thead>
            <tr>
              <th style="width: 100px;">难度阶梯</th>
              <th style="width: 260px;">任务定义与典型用例</th>
              <th v-for="model in models" :key="model.model_id" class="model-col-header">
                {{ model.model_name || model.model_id }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(info, tierKey) in TIER_DEFINITIONS" :key="tierKey">
              <td>
                <span class="badge" :class="'badge-tier-' + tierKey.toLowerCase()">
                  {{ tierKey }} 阶梯
                </span>
              </td>
              <td>
                <div class="tier-desc-cell">
                  <strong>{{ info.name }}</strong>
                  <p>{{ info.desc }}</p>
                </div>
              </td>
              <td v-for="model in models" :key="model.model_id">
                <div class="model-tier-cell">
                  <div class="cell-primary-score">
                    <span class="pass-rate-val">
                      {{ ((model.tier_breakdown?.[tierKey]?.accuracy || 0) * 100).toFixed(1) }}%
                    </span>
                    <span class="pass-count">
                      ({{ model.tier_breakdown?.[tierKey]?.passed || 0 }}/{{ model.tier_breakdown?.[tierKey]?.total || 0 }})
                    </span>
                  </div>
                  <div class="cell-progress">
                    <div
                      class="cell-fill"
                      :style="{
                        width: ((model.tier_breakdown?.[tierKey]?.accuracy || 0) * 100) + '%',
                        backgroundColor: info.color
                      }"
                    ></div>
                  </div>
                </div>
              </td>
            </tr>
            <!-- Frontier Summary Row -->
            <tr class="frontier-summary-row">
              <td>
                <span class="badge badge-purple">⚡ L4+L5 前沿</span>
              </td>
              <td>
                <div class="tier-desc-cell">
                  <strong>真实极限能力分水岭</strong>
                  <p>排除基础通识干扰，体现自主工具编排排障与高难度代码重构</p>
                </div>
              </td>
              <td v-for="model in models" :key="model.model_id">
                <div class="model-tier-cell">
                  <div class="cell-primary-score">
                    <span class="pass-rate-val highlight-val">
                      {{ (model.l4_l5_frontier_accuracy * 100).toFixed(1) }}%
                    </span>
                  </div>
                  <div class="cell-progress">
                    <div
                      class="cell-fill"
                      :style="{
                        width: (model.l4_l5_frontier_accuracy * 100) + '%',
                        backgroundColor: '#a855f7'
                      }"
                    ></div>
                  </div>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, watch, nextTick, onBeforeUnmount } from 'vue';
import Chart from 'chart.js/auto';
import { TIER_DEFINITIONS } from '../utils/benchmark';

const props = defineProps({
  models: {
    type: Array,
    required: true
  }
});

const tierChartRef = ref(null);
let tierChartInstance = null;

const COLOR_PALETTE = ['#6366f1', '#10b981', '#f59e0b', '#0ea5e9', '#a855f7', '#f43f5e'];

function updateTierChart() {
  if (!tierChartRef.value) return;
  if (tierChartInstance) tierChartInstance.destroy();

  const labels = Object.keys(TIER_DEFINITIONS).map(k => `${k} (${TIER_DEFINITIONS[k].name.split(' ')[1]})`);

  const datasets = props.models.map((m, idx) => {
    const color = COLOR_PALETTE[idx % COLOR_PALETTE.length];
    const data = Object.keys(TIER_DEFINITIONS).map(k => {
      const acc = (m.tier_breakdown?.[k]?.accuracy || 0) * 100;
      return Math.round(acc * 10) / 10;
    });

    return {
      label: m.model_name || m.model_id,
      data,
      backgroundColor: color,
      borderRadius: 6,
      barPercentage: 0.75,
      categoryPercentage: 0.8
    };
  });

  tierChartInstance = new Chart(tierChartRef.value, {
    type: 'bar',
    data: { labels, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        y: {
          min: 60,
          max: 100,
          grid: { color: 'rgba(255, 255, 255, 0.08)' },
          ticks: {
            color: '#94a3b8',
            callback: (v) => v + '%'
          }
        },
        x: {
          grid: { display: false },
          ticks: {
            color: '#cbd5e1',
            font: { size: 12, weight: '600' }
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

watch(() => props.models, () => {
  nextTick(() => {
    updateTierChart();
  });
}, { immediate: true });

onMounted(() => {
  updateTierChart();
});

onBeforeUnmount(() => {
  if (tierChartInstance) tierChartInstance.destroy();
});
</script>

<style scoped>
.tier-breakdown-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.tier-header-card {
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
  max-width: 720px;
}

.frontier-badges {
  display: flex;
  gap: 0.75rem;
}

.frontier-stat-pill {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  padding: 0.5rem 0.85rem;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.frontier-stat-pill.highlight {
  border-color: var(--accent-purple);
  background: rgba(168, 85, 247, 0.08);
}

.pill-label {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.pill-value {
  font-size: 1.1rem;
  font-weight: 800;
  color: var(--text-heading);
  font-family: var(--font-mono);
}

.frontier-stat-pill.highlight .pill-value {
  color: var(--accent-purple);
}

/* Tier Cards Grid */
.tier-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 1rem;
}

.tier-info-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  position: relative;
  overflow: hidden;
}

.tier-info-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
}

.card-l1::before { background: var(--accent-sky); }
.card-l2::before { background: var(--accent-emerald); }
.card-l3::before { background: var(--accent-amber); }
.card-l4::before { background: var(--accent-purple); }
.card-l5::before { background: var(--accent-rose); }

.tier-card-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.tier-level-title {
  font-weight: 700;
  font-size: 0.88rem;
  color: var(--text-heading);
}

.tier-card-desc {
  font-size: 0.75rem;
  color: var(--text-muted);
  line-height: 1.35;
  min-height: 48px;
}

.tier-model-ranks {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  border-top: 1px dashed var(--border-soft);
  padding-top: 0.6rem;
}

.tier-model-score-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.75rem;
}

.model-abbr {
  width: 70px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text-body);
  font-weight: 600;
}

.score-track {
  flex: 1;
  height: 6px;
  background: var(--bg-subtle);
  border-radius: 3px;
  overflow: hidden;
}

.score-fill {
  height: 100%;
  border-radius: 3px;
}

.score-percent {
  width: 44px;
  text-align: right;
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--text-heading);
}

/* Chart Section */
.chart-section-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
}

.chart-header {
  margin-bottom: 1.25rem;
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

.chart-canvas-wrapper {
  position: relative;
  height: 380px;
  width: 100%;
}

/* Detailed Matrix Table */
.tier-table-card {
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

.tier-matrix-table th {
  background: var(--bg-surface);
  color: var(--text-muted);
  font-size: 0.8rem;
  padding: 0.75rem 1rem;
}

.tier-matrix-table td {
  padding: 0.85rem 1rem;
  border-bottom: 1px solid var(--border-soft);
  vertical-align: middle;
}

.tier-desc-cell strong {
  display: block;
  color: var(--text-heading);
  font-size: 0.85rem;
  margin-bottom: 0.15rem;
}

.tier-desc-cell p {
  color: var(--text-muted);
  font-size: 0.78rem;
  line-height: 1.3;
}

.model-tier-cell {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.cell-primary-score {
  display: flex;
  align-items: baseline;
  gap: 0.4rem;
}

.pass-rate-val {
  font-family: var(--font-mono);
  font-weight: 800;
  font-size: 0.95rem;
  color: var(--text-heading);
}

.pass-count {
  font-size: 0.75rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
}

.cell-progress {
  height: 6px;
  background: var(--bg-subtle);
  border-radius: 3px;
  overflow: hidden;
}

.cell-fill {
  height: 100%;
  border-radius: 3px;
}

.frontier-summary-row {
  background: rgba(168, 85, 247, 0.05);
}

.frontier-summary-row td {
  border-bottom: none;
  font-weight: 700;
}

.highlight-val {
  color: var(--accent-purple);
  font-size: 1.05rem;
}
</style>
