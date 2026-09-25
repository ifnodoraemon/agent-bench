<template>
  <div class="app-root" @dragover.prevent="isDragging = true" @dragleave.prevent="isDragging = false" @drop.prevent="handleFileDrop">
    <!-- Drag & Drop Overlay -->
    <div v-if="isDragging" class="drag-overlay">
      <div class="drag-card">
        <div class="drag-icon">📥</div>
        <h3>松开鼠标以导入评测结果 JSON 文件</h3>
        <p>支持 agent-bench 产出的 eval_results_*.json 完整报告</p>
      </div>
    </div>

    <!-- Top Navigation Header -->
    <header class="app-header">
      <div class="header-container">
        <div class="brand-block">
          <div class="brand-logo">
            <span class="logo-emoji">🦀</span>
            <div class="logo-text">
              <span class="logo-title">AGENT-BENCH</span>
              <span class="logo-badge">v2.4.0</span>
            </div>
          </div>
          <span class="brand-divider">/</span>
          <span class="brand-desc">高性能 Rust 异步多维智能体评测与天梯可视化套件</span>
        </div>

        <div class="header-actions">
          <!-- Live Run History Selector (from Rust /api/runs) -->
          <div v-if="availableRuns.length > 0" class="run-history-picker">
            <label class="picker-label">历史报告:</label>
            <select
              v-model="selectedRunFile"
              class="run-select"
              @change="handleRunFileChange"
              title="切换加载历史评测报告"
            >
              <option v-for="r in availableRuns" :key="r.filename" :value="r.filename">
                {{ r.timestamp }} ({{ r.models.join(', ') || r.size_human }})
              </option>
            </select>
          </div>

          <!-- Hidden File Input -->
          <input
            ref="fileInputRef"
            type="file"
            accept=".json"
            style="display: none"
            @change="handleFileInput"
          />

          <button class="btn btn-header" @click="triggerFileInput" title="上传本地测试结果 JSON">
            📂 上传报告
          </button>

          <button class="btn btn-header btn-ghost" @click="loadDefaultDataset" title="重新载入默认 1009 题样例">
            🔄 重置样例
          </button>

          <!-- Theme Toggle -->
          <button class="btn-theme-toggle" @click="toggleTheme" :title="'切换至 ' + (currentTheme === 'dark' ? '暖色浅色' : '极客深色') + ' 模式'">
            {{ currentTheme === 'dark' ? '☀️ 浅色' : '🌙 深色' }}
          </button>
        </div>
      </div>
    </header>

    <!-- Main Content Area -->
    <main class="main-container">
      <!-- Loading State -->
      <div v-if="isLoading" class="loading-state">
        <div class="spinner"></div>
        <p>正在解析评测报告与运算 Elo 天梯数据...</p>
      </div>

      <!-- Error State -->
      <div v-else-if="errorMessage" class="error-banner">
        <span class="error-icon">⚠️</span>
        <div class="error-content">
          <strong>数据解析错误</strong>
          <p>{{ errorMessage }}</p>
        </div>
        <button class="btn btn-secondary btn-sm" @click="loadDefaultDataset">重置为内置样例</button>
      </div>

      <!-- Loaded Dashboard -->
      <div v-else class="dashboard-content">
        <!-- Top Global KPI Cards Ribbon -->
        <div class="kpi-ribbon-grid">
          <div class="kpi-card rank-first-card">
            <div class="kpi-icon-wrap">🥇</div>
            <div class="kpi-body">
              <span class="kpi-label">天梯榜首模型</span>
              <span class="kpi-value highlight-gold">{{ topModel?.model_name || '-' }}</span>
              <span class="kpi-sub">Elo: {{ Math.round(topModel?.computed_elo || topModel?.elo_rating || 1200) }} 分</span>
            </div>
          </div>

          <div class="kpi-card">
            <div class="kpi-icon-wrap">⚡</div>
            <div class="kpi-body">
              <span class="kpi-label">最高极限前沿 (L4+L5)</span>
              <span class="kpi-value text-purple">{{ topFrontierModel ? ((topFrontierModel.l4_l5_frontier_accuracy * 100).toFixed(1) + '%') : '-' }}</span>
              <span class="kpi-sub">{{ topFrontierModel?.model_name || '-' }}</span>
            </div>
          </div>

          <div class="kpi-card">
            <div class="kpi-icon-wrap">🎯</div>
            <div class="kpi-body">
              <span class="kpi-label">最高微观准确率 (Micro)</span>
              <span class="kpi-value text-emerald">{{ topAccModel ? ((topAccModel.overall_accuracy * 100).toFixed(1) + '%') : '-' }}</span>
              <span class="kpi-sub">{{ topAccModel?.model_name || '-' }}</span>
            </div>
          </div>

          <div class="kpi-card">
            <div class="kpi-icon-wrap">🚀</div>
            <div class="kpi-body">
              <span class="kpi-label">最高推理吞吐 (TPS)</span>
              <span class="kpi-value text-sky">{{ maxTpsModel ? (maxTpsModel.avg_tps.toFixed(1) + ' tps') : '-' }}</span>
              <span class="kpi-sub">{{ maxTpsModel?.model_name || '-' }}</span>
            </div>
          </div>

          <div class="kpi-card">
            <div class="kpi-icon-wrap">⏱️</div>
            <div class="kpi-body">
              <span class="kpi-label">最低端到端延迟</span>
              <span class="kpi-value text-amber">{{ minLatencyModel ? (Math.round(minLatencyModel.avg_latency_ms) + ' ms') : '-' }}</span>
              <span class="kpi-sub">{{ minLatencyModel?.model_name || '-' }}</span>
            </div>
          </div>

          <div class="kpi-card">
            <div class="kpi-icon-wrap">🧪</div>
            <div class="kpi-body">
              <span class="kpi-label">参评模型与用例规模</span>
              <span class="kpi-value text-heading">{{ enrichedModels.length }} 个模型</span>
              <span class="kpi-sub">共 {{ totalTestCases }} 题次完整测试</span>
            </div>
          </div>
        </div>

        <!-- Tab Navigation Bar -->
        <div class="dashboard-tabs-bar">
          <button
            class="tab-btn"
            :class="{ active: currentTab === 'leaderboard' }"
            @click="currentTab = 'leaderboard'"
          >
            <span class="tab-icon">🏆</span>
            <span class="tab-title">综合天梯榜</span>
          </button>

          <button
            class="tab-btn"
            :class="{ active: currentTab === 'radar' }"
            @click="currentTab = 'radar'"
          >
            <span class="tab-icon">🕸️</span>
            <span class="tab-title">领域多维雷达</span>
          </button>

          <button
            class="tab-btn"
            :class="{ active: currentTab === 'tiers' }"
            @click="currentTab = 'tiers'"
          >
            <span class="tab-icon">🪜</span>
            <span class="tab-title">难度阶梯 (L1~L5)</span>
          </button>

          <button
            class="tab-btn"
            :class="{ active: currentTab === 'pareto' }"
            @click="currentTab = 'pareto'"
          >
            <span class="tab-icon">📈</span>
            <span class="tab-title">能效与帕累托前沿</span>
          </button>

          <button
            class="tab-btn"
            :class="{ active: currentTab === 'inspector' }"
            @click="currentTab = 'inspector'"
          >
            <span class="tab-icon">🔍</span>
            <span class="tab-title">轨迹与坏例探查</span>
          </button>

          <button
            class="tab-btn"
            :class="{ active: currentTab === 'battle' }"
            @click="currentTab = 'battle'"
          >
            <span class="tab-icon">⚔️</span>
            <span class="tab-title">双模型对决擂台</span>
          </button>

          <button
            class="tab-btn"
            :class="{ active: currentTab === 'diagnostics' }"
            @click="currentTab = 'diagnostics'"
          >
            <span class="tab-icon">💡</span>
            <span class="tab-title">自诊断与洞见</span>
          </button>

          <button
            class="tab-btn highlight-tab"
            :class="{ active: currentTab === 'live' }"
            @click="currentTab = 'live'"
          >
            <span class="tab-icon">🚀</span>
            <span class="tab-title">实时评测控制台</span>
          </button>
        </div>

        <!-- Tab Views -->
        <div class="tab-view-container">
          <!-- 1. Leaderboard -->
          <div v-show="currentTab === 'leaderboard'">
            <LeaderboardTable
              :models="enrichedModels"
              :selected-model-id="selectedModelForInspector"
              @select-model="handleModelSelect"
            />
          </div>

          <!-- 2. Domain Radar -->
          <div v-show="currentTab === 'radar'">
            <RadarAnalysis :models="enrichedModels" />
          </div>

          <!-- 3. Tier Breakdown -->
          <div v-show="currentTab === 'tiers'">
            <TierBreakdown :models="enrichedModels" />
          </div>

          <!-- 4. Pareto Frontier -->
          <div v-show="currentTab === 'pareto'">
            <ParetoAnalysis :models="enrichedModels" />
          </div>

          <!-- 5. Case Inspector -->
          <div v-show="currentTab === 'inspector'">
            <CaseInspector
              :models="enrichedModels"
              :initial-model-id="selectedModelForInspector"
            />
          </div>

          <!-- 6. Head to Head Battle Arena -->
          <div v-show="currentTab === 'battle'">
            <ModelComparison :models="enrichedModels" />
          </div>

          <!-- 7. Diagnostics -->
          <div v-show="currentTab === 'diagnostics'">
            <DiagnosticInsights :models="enrichedModels" />
          </div>

          <!-- 8. Live Benchmark Runner -->
          <div v-show="currentTab === 'live'">
            <LiveBenchmarkRunner @load-run="handleLoadNewRun" />
          </div>
        </div>
      </div>
    </main>

    <!-- App Footer -->
    <footer class="app-footer">
      <div class="footer-content">
        <p>
          <strong>Agent-Bench</strong> — 基于 Rust 构建的高性能异步智能体基准评测框架 (Tokio · Axum · Reqwest · Chart.js · Vue 3)
        </p>
        <p class="footer-links">
          <span>支持多维度指标建模：目标达成度、工具规范、思维链深度、异常自愈与执行能效</span>
        </p>
      </div>
    </footer>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue';
import LeaderboardTable from './components/LeaderboardTable.vue';
import RadarAnalysis from './components/RadarAnalysis.vue';
import TierBreakdown from './components/TierBreakdown.vue';
import ParetoAnalysis from './components/ParetoAnalysis.vue';
import CaseInspector from './components/CaseInspector.vue';
import ModelComparison from './components/ModelComparison.vue';
import DiagnosticInsights from './components/DiagnosticInsights.vue';
import LiveBenchmarkRunner from './components/LiveBenchmarkRunner.vue';
import { computeHeadToHeadElo } from './utils/elo';
import { enrichModelSummary } from './utils/benchmark';

// Theme state
const currentTheme = ref('dark');

function toggleTheme() {
  const next = currentTheme.value === 'dark' ? 'warm' : 'dark';
  currentTheme.value = next;
  document.documentElement.setAttribute('data-theme', next);
  try {
    localStorage.setItem('agent_bench_theme', next);
  } catch (e) {}
}

// Active Tab
const currentTab = ref('leaderboard');
const selectedModelForInspector = ref('');

// Data state
const rawModels = ref([]);
const isLoading = ref(true);
const errorMessage = ref('');
const isDragging = ref(false);
const fileInputRef = ref(null);

// Available historical runs from Rust backend
const availableRuns = ref([]);
const selectedRunFile = ref('');

// Processed models with Elo and Tiers
const enrichedModels = computed(() => {
  if (!rawModels.value || rawModels.value.length === 0) return [];
  const withElo = computeHeadToHeadElo(rawModels.value);
  return withElo.map(enrichModelSummary);
});

// KPI Highlights
const topModel = computed(() => {
  if (enrichedModels.value.length === 0) return null;
  return [...enrichedModels.value].sort((a, b) => {
    const eloA = a.computed_elo || a.elo_rating || 1200;
    const eloB = b.computed_elo || b.elo_rating || 1200;
    return eloB - eloA;
  })[0];
});

const topFrontierModel = computed(() => {
  if (enrichedModels.value.length === 0) return null;
  return [...enrichedModels.value].sort((a, b) => b.l4_l5_frontier_accuracy - a.l4_l5_frontier_accuracy)[0];
});

const topAccModel = computed(() => {
  if (enrichedModels.value.length === 0) return null;
  return [...enrichedModels.value].sort((a, b) => b.overall_accuracy - a.overall_accuracy)[0];
});

const maxTpsModel = computed(() => {
  if (enrichedModels.value.length === 0) return null;
  return [...enrichedModels.value].sort((a, b) => (b.avg_tps || 0) - (a.avg_tps || 0))[0];
});

const minLatencyModel = computed(() => {
  if (enrichedModels.value.length === 0) return null;
  return [...enrichedModels.value]
    .filter(m => m.avg_latency_ms > 0)
    .sort((a, b) => a.avg_latency_ms - b.avg_latency_ms)[0];
});

const totalTestCases = computed(() => {
  return enrichedModels.value.reduce((acc, m) => acc + (m.total_cases || m.case_results?.length || 0), 0);
});

function handleModelSelect(model) {
  selectedModelForInspector.value = model.model_id;
  currentTab.value = 'inspector';
}

function triggerFileInput() {
  if (fileInputRef.value) {
    fileInputRef.value.click();
  }
}

function parseAndLoadJSON(jsonString) {
  try {
    const parsed = JSON.parse(jsonString);
    let list = [];
    if (Array.isArray(parsed)) {
      list = parsed;
    } else if (parsed.summaries && Array.isArray(parsed.summaries)) {
      list = parsed.summaries;
    } else if (parsed.models && Array.isArray(parsed.models)) {
      list = parsed.models;
    } else if (parsed.model_id) {
      list = [parsed];
    } else {
      throw new Error('未在 JSON 中找到合法的模型评测数据结构');
    }

    rawModels.value = list;
    if (list.length > 0) {
      selectedModelForInspector.value = list[0].model_id;
    }
    errorMessage.value = '';
  } catch (err) {
    errorMessage.value = `文件解析失败: ${err.message}`;
  }
}

function handleFileInput(e) {
  const file = e.target.files?.[0];
  if (!file) return;
  readFile(file);
  e.target.value = '';
}

function handleFileDrop(e) {
  isDragging.value = false;
  const file = e.dataTransfer.files?.[0];
  if (!file) return;
  readFile(file);
}

function readFile(file) {
  isLoading.value = true;
  const reader = new FileReader();
  reader.onload = (event) => {
    parseAndLoadJSON(event.target.result);
    isLoading.value = false;
  };
  reader.onerror = () => {
    errorMessage.value = '读取本地文件出错';
    isLoading.value = false;
  };
  reader.readAsText(file);
}

async function loadDefaultDataset() {
  isLoading.value = true;
  errorMessage.value = '';
  try {
    const res = await fetch('/data/default_results.json');
    if (!res.ok) {
      throw new Error(`HTTP ${res.status} 载入默认数据失败`);
    }
    const data = await res.json();
    rawModels.value = Array.isArray(data) ? data : (data.summaries || [data]);
    if (rawModels.value.length > 0) {
      selectedModelForInspector.value = rawModels.value[0].model_id;
    }
  } catch (err) {
    errorMessage.value = `无法载入样例数据: ${err.message}`;
  } finally {
    isLoading.value = false;
  }
}

async function fetchAvailableRuns() {
  try {
    const res = await fetch('/api/runs');
    if (res.ok) {
      const runs = await res.json();
      if (Array.isArray(runs) && runs.length > 0) {
        availableRuns.value = runs;
        selectedRunFile.value = runs[0].filename;
      }
    }
  } catch (e) {
    // Running in static preview without Rust backend, gracefully ignore
  }
}

async function handleRunFileChange() {
  if (!selectedRunFile.value) return;
  isLoading.value = true;
  try {
    const res = await fetch(`/api/runs/${selectedRunFile.value}`);
    if (!res.ok) {
      throw new Error(`HTTP ${res.status} 加载历史文件失败`);
    }
    const data = await res.json();
    rawModels.value = Array.isArray(data) ? data : (data.summaries || [data]);
    if (rawModels.value.length > 0) {
      selectedModelForInspector.value = rawModels.value[0].model_id;
    }
  } catch (err) {
    errorMessage.value = err.message;
  } finally {
    isLoading.value = false;
  }
}

async function handleLoadNewRun(filename) {
  await fetchAvailableRuns();
  selectedRunFile.value = filename;
  await handleRunFileChange();
  currentTab.value = 'leaderboard';
}

onMounted(() => {
  // Load saved theme
  try {
    const saved = localStorage.getItem('agent_bench_theme');
    if (saved === 'warm' || saved === 'dark') {
      currentTheme.value = saved;
      document.documentElement.setAttribute('data-theme', saved);
    }
  } catch (e) {}

  loadDefaultDataset();
  fetchAvailableRuns();
});
</script>

<style scoped>
.app-root {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  position: relative;
}

/* Drag overlay */
.drag-overlay {
  position: fixed;
  inset: 0;
  background: rgba(9, 13, 22, 0.85);
  backdrop-filter: blur(8px);
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
}

.drag-card {
  background: var(--bg-card);
  border: 2px dashed var(--primary);
  border-radius: 16px;
  padding: 3rem 4rem;
  text-align: center;
}

.drag-icon {
  font-size: 3.5rem;
  margin-bottom: 1rem;
}

.drag-card h3 {
  font-size: 1.35rem;
  color: var(--text-heading);
  margin-bottom: 0.5rem;
}

.drag-card p {
  color: var(--text-muted);
  font-size: 0.9rem;
}

/* Header */
.app-header {
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-soft);
  position: sticky;
  top: 0;
  z-index: 100;
  backdrop-filter: blur(12px);
}

.header-container {
  max-width: 1440px;
  margin: 0 auto;
  padding: 0.85rem 1.5rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}

.brand-block {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.brand-logo {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.logo-emoji {
  font-size: 1.8rem;
  line-height: 1;
}

.logo-text {
  display: flex;
  align-items: baseline;
  gap: 0.4rem;
}

.logo-title {
  font-weight: 900;
  font-size: 1.25rem;
  letter-spacing: -0.03em;
  background: linear-gradient(135deg, #818cf8 0%, #c084fc 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.logo-badge {
  font-size: 0.72rem;
  font-weight: 700;
  background: var(--bg-subtle);
  color: var(--primary);
  border: 1px solid var(--border-soft);
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
  font-family: var(--font-mono);
}

.brand-divider {
  color: var(--border-strong);
  font-size: 1.1rem;
}

.brand-desc {
  font-size: 0.82rem;
  color: var(--text-muted);
}

@media (max-width: 900px) {
  .brand-desc {
    display: none;
  }
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.run-history-picker {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  padding: 0.2rem 0.6rem;
}

.picker-label {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-weight: 700;
}

.run-select {
  background: transparent;
  border: none;
  color: var(--text-heading);
  font-size: 0.78rem;
  font-family: var(--font-sans);
  font-weight: 600;
  outline: none;
  cursor: pointer;
  max-width: 200px;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.85rem;
  border-radius: 8px;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-header {
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  color: var(--text-heading);
}

.btn-header:hover {
  background: var(--bg-hover);
  border-color: var(--primary);
}

.btn-ghost {
  background: transparent;
  border-color: transparent;
  color: var(--text-muted);
}

.btn-ghost:hover {
  background: var(--bg-subtle);
  color: var(--text-heading);
}

.btn-theme-toggle {
  background: var(--bg-subtle);
  border: 1px solid var(--border-soft);
  color: var(--text-heading);
  padding: 0.45rem 0.75rem;
  border-radius: 8px;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-theme-toggle:hover {
  background: var(--bg-hover);
}

/* Main Container */
.main-container {
  max-width: 1440px;
  width: 100%;
  margin: 0 auto;
  padding: 1.5rem;
  flex: 1;
  display: flex;
  flex-direction: column;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 5rem 1rem;
  gap: 1.25rem;
  color: var(--text-muted);
}

.spinner {
  width: 44px;
  height: 44px;
  border: 3px solid var(--border-soft);
  border-top-color: var(--primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error-banner {
  background: var(--rose-bg);
  border: 1px solid var(--rose-border);
  border-radius: 10px;
  padding: 1.25rem 1.5rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  color: var(--accent-rose);
  margin-bottom: 1.5rem;
}

.error-icon {
  font-size: 1.8rem;
}

.error-content strong {
  display: block;
  font-size: 0.95rem;
}

.error-content p {
  font-size: 0.85rem;
  margin-top: 0.2rem;
}

/* KPI Ribbon */
.kpi-ribbon-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.kpi-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.1rem 1.25rem;
  display: flex;
  align-items: center;
  gap: 0.9rem;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.kpi-card:hover {
  transform: translateY(-2px);
  border-color: var(--border-strong);
}

.rank-first-card {
  border-color: rgba(245, 158, 11, 0.35);
  background: linear-gradient(135deg, var(--bg-card) 60%, rgba(245, 158, 11, 0.08) 100%);
}

.kpi-icon-wrap {
  font-size: 2rem;
  line-height: 1;
}

.kpi-body {
  display: flex;
  flex-direction: column;
}

.kpi-label {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-weight: 600;
}

.kpi-value {
  font-family: var(--font-mono);
  font-size: 1.15rem;
  font-weight: 800;
  margin: 0.15rem 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 150px;
}

.highlight-gold {
  color: #fbbf24;
}

.text-purple { color: #c084fc; }
.text-emerald { color: #34d399; }
.text-sky { color: #38bdf8; }
.text-amber { color: #fbbf24; }
.text-heading { color: var(--text-heading); }

.kpi-sub {
  font-size: 0.72rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 150px;
}

/* Tabs Bar */
.dashboard-tabs-bar {
  display: flex;
  gap: 0.4rem;
  background: var(--bg-surface);
  padding: 0.35rem;
  border-radius: 12px;
  border: 1px solid var(--border-soft);
  margin-bottom: 1.5rem;
  overflow-x: auto;
}

.tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  background: transparent;
  border: none;
  color: var(--text-muted);
  padding: 0.65rem 1.15rem;
  border-radius: 8px;
  font-size: 0.88rem;
  font-weight: 700;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.tab-btn:hover {
  background: var(--bg-card);
  color: var(--text-heading);
}

.tab-btn.active {
  background: var(--primary);
  color: #ffffff;
  box-shadow: 0 4px 12px var(--primary-glow);
}

.tab-btn.highlight-tab {
  border: 1px solid rgba(99, 102, 241, 0.4);
  background: rgba(99, 102, 241, 0.08);
}
.tab-btn.highlight-tab.active {
  background: var(--primary);
  border-color: transparent;
}

.tab-icon {
  font-size: 1rem;
}

/* View Container */
.tab-view-container {
  display: flex;
  flex-direction: column;
}

/* Footer */
.app-footer {
  border-top: 1px solid var(--border-soft);
  background: var(--bg-surface);
  padding: 1.5rem;
  text-align: center;
  font-size: 0.8rem;
  color: var(--text-muted);
  margin-top: 3rem;
}

.footer-content {
  max-width: 1440px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.footer-links {
  font-size: 0.75rem;
  color: var(--text-faint);
}
</style>
