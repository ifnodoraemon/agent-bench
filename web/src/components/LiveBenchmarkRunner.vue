<template>
  <div class="live-runner-container">
    <!-- Header Card -->
    <div class="runner-header-card">
      <div class="header-left">
        <h3 class="panel-title">🚀 实时评测调度控制台 (Live Benchmark Runner)</h3>
        <p class="panel-subtitle">
          交互式配置任务子集、难度阶梯与并发线程，向 Rust 评测引擎发起实时基准测试，毫秒级流式观察智能体执行状态与实时通过率。
        </p>
      </div>

      <div class="status-indicator-box">
        <span class="status-dot" :class="'dot-' + liveState.status"></span>
        <span class="status-text">{{ getStatusText(liveState.status) }}</span>
      </div>
    </div>

    <!-- Configuration Form Card -->
    <div class="config-card">
      <div class="config-grid">
        <!-- Model Selection -->
        <div class="config-item">
          <label>评测模型:</label>
          <select v-model="formModel" :disabled="isRunning" class="cfg-select">
            <option value="mock-pro">🤖 Mock-Pro-v1 (模拟深度思考与工具编排)</option>
            <option value="mock-fast">⚡ Mock-Fast-v1 (模拟极速轻量模型)</option>
          </select>
        </div>

        <!-- Category -->
        <div class="config-item">
          <label>任务分类:</label>
          <select v-model="formCategory" :disabled="isRunning" class="cfg-select">
            <option value="all">全部分类 (全部 18 类任务)</option>
            <option v-for="(name, catKey) in CATEGORY_NAMES" :key="catKey" :value="catKey">
              {{ name }}
            </option>
          </select>
        </div>

        <!-- Difficulty Tier -->
        <div class="config-item">
          <label>难度分级:</label>
          <select v-model="formDifficulty" :disabled="isRunning" class="cfg-select">
            <option value="all">全部阶梯 (L1 ~ L5)</option>
            <option value="frontier">⚡ 极限前沿 (L4 + L5)</option>
            <option value="L1">L1 基础自洽</option>
            <option value="L2">L2 标准任务</option>
            <option value="L3">L3 专业领域</option>
            <option value="L4">L4 自主智能体</option>
            <option value="L5">L5 极限难题</option>
          </select>
        </div>

        <!-- Limit -->
        <div class="config-item">
          <label>抽取测试用例数:</label>
          <select v-model="formLimit" :disabled="isRunning" class="cfg-select">
            <option :value="5">5 题 (秒级快速验证)</option>
            <option :value="10">10 题 (标准抽样)</option>
            <option :value="25">25 题 (均衡切片)</option>
            <option :value="50">50 题 (深度基准)</option>
            <option :value="200">200 题 (大规模压测)</option>
          </select>
        </div>

        <!-- Concurrency -->
        <div class="config-item">
          <label>并发工作线程 ({{ formConcurrency }}):</label>
          <input
            v-model.number="formConcurrency"
            type="range"
            min="1"
            max="16"
            :disabled="isRunning"
            class="cfg-range"
          />
        </div>
      </div>

      <!-- Control Buttons -->
      <div class="config-actions">
        <button
          v-if="!isRunning"
          class="btn btn-primary btn-start"
          @click="startBenchmark"
        >
          ▶ 启动实时评测流水线
        </button>
        <button
          v-else
          class="btn btn-danger btn-stop"
          @click="stopBenchmark"
        >
          ⏹ 终止当前评测
        </button>
      </div>
    </div>

    <!-- Live Execution Monitor -->
    <div v-if="liveState.status !== 'idle'" class="monitor-card">
      <div class="monitor-header">
        <div class="monitor-title">
          <h4>实时评测流水线看板</h4>
          <span class="monitor-sub">{{ liveState.message }}</span>
        </div>
        <div v-if="liveState.elapsed_secs > 0" class="time-tag">
          耗时 {{ liveState.elapsed_secs }}s
        </div>
      </div>

      <!-- Progress Bar -->
      <div class="progress-section">
        <div class="progress-labels">
          <span>进度: {{ liveState.completed_cases }} / {{ liveState.total_cases }} 题</span>
          <span class="percent-lbl">{{ progressPercent }}%</span>
        </div>
        <div class="progress-track">
          <div
            class="progress-fill"
            :style="{ width: progressPercent + '%' }"
            :class="{ 'fill-completed': liveState.status === 'completed' }"
          ></div>
        </div>
      </div>

      <!-- Live KPI Ribbon -->
      <div class="live-kpi-grid">
        <div class="live-kpi-card">
          <span class="kpi-lbl">实时通过率</span>
          <span class="kpi-val text-emerald">{{ livePassRate }}%</span>
          <span class="kpi-sub">{{ liveState.passed_cases }} 通过 / {{ liveState.completed_cases }} 完成</span>
        </div>

        <div class="live-kpi-card">
          <span class="kpi-lbl">待处理用例</span>
          <span class="kpi-val text-amber">{{ Math.max(0, liveState.total_cases - liveState.completed_cases) }}</span>
          <span class="kpi-sub">总计 {{ liveState.total_cases }} 题</span>
        </div>

        <div class="live-kpi-card">
          <span class="kpi-lbl">当前运行模型</span>
          <span class="kpi-val text-sky">{{ liveState.model_name }}</span>
          <span class="kpi-sub">并发: {{ formConcurrency }} 线程</span>
        </div>
      </div>

      <!-- On Completion Action Banner -->
      <div v-if="liveState.status === 'completed' && liveState.result_file" class="completion-banner">
        <div class="banner-left">
          <span class="banner-icon">🎉</span>
          <div>
            <strong>本次评测已圆满结束并落盘保存！</strong>
            <p>结果文件已持久化至: {{ liveState.result_file }}</p>
          </div>
        </div>
        <button class="btn btn-success" @click="$emit('load-run', liveState.result_file)">
          📊 立即在全局看板中查看完整分析
        </button>
      </div>

      <!-- Live Finished Cases Stream -->
      <div v-if="liveState.live_cases.length > 0" class="live-cases-section">
        <div class="section-title">
          <h5>已完成用例实时流水 (最新 30 例)</h5>
        </div>

        <div class="live-cases-table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th>用例 ID</th>
                <th>任务分类</th>
                <th>评测状态</th>
                <th>得分</th>
                <th>单题延迟</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="c in liveState.live_cases" :key="c.id">
                <td class="case-id-cell">{{ c.id }}</td>
                <td>
                  <span class="badge badge-info">{{ CATEGORY_NAMES[c.category] || c.category }}</span>
                </td>
                <td>
                  <span class="badge" :class="c.passed ? 'badge-success' : 'badge-danger'">
                    {{ c.passed ? '✓ 通过' : '✕ 失败' }}
                  </span>
                </td>
                <td class="font-mono">{{ (c.score || 0).toFixed(2) }}</td>
                <td class="font-mono text-muted">{{ Math.round(c.latency_ms || 0) }} ms</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { CATEGORY_NAMES } from '../utils/benchmark';

const emit = defineEmits(['load-run']);

const formModel = ref('mock-pro');
const formCategory = ref('all');
const formDifficulty = ref('all');
const formLimit = ref(10);
const formConcurrency = ref(4);

const liveState = ref({
  status: 'idle',
  model_name: 'Mock-Pro-v1',
  total_cases: 0,
  completed_cases: 0,
  passed_cases: 0,
  live_cases: [],
  elapsed_secs: 0,
  result_file: null,
  message: null
});

let pollTimer = null;

const isRunning = computed(() => liveState.value.status === 'running');

const progressPercent = computed(() => {
  if (!liveState.value.total_cases) return 0;
  return Math.min(100, Math.round((liveState.value.completed_cases / liveState.value.total_cases) * 100));
});

const livePassRate = computed(() => {
  if (!liveState.value.completed_cases) return 0;
  return ((liveState.value.passed_cases / liveState.value.completed_cases) * 100).toFixed(1);
});

function getStatusText(st) {
  switch (st) {
    case 'running': return '正在执行评测...';
    case 'completed': return '评测已完成';
    case 'error': return '评测发生异常';
    default: return '待命中 (Ready)';
  }
}

async function startBenchmark() {
  try {
    const payload = {
      model: formModel.value,
      category: formCategory.value === 'all' ? null : formCategory.value,
      difficulty: formDifficulty.value === 'all' ? null : formDifficulty.value,
      limit: formLimit.value,
      concurrency: formConcurrency.value
    };

    const res = await fetch('/api/benchmark/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    if (!res.ok) {
      const err = await res.json();
      alert(`无法启动评测: ${err.message || res.statusText}`);
      return;
    }

    startPolling();
  } catch (err) {
    alert(`网络请求错误: ${err.message}`);
  }
}

async function stopBenchmark() {
  try {
    await fetch('/api/benchmark/stop', { method: 'POST' });
    stopPolling();
    liveState.value.status = 'idle';
  } catch (err) {}
}

async function fetchStatus() {
  try {
    const res = await fetch('/api/benchmark/status');
    if (res.ok) {
      const data = await res.json();
      liveState.value = data;
      if (data.status !== 'running') {
        stopPolling();
      }
    }
  } catch (err) {
    stopPolling();
  }
}

function startPolling() {
  stopPolling();
  fetchStatus();
  pollTimer = setInterval(fetchStatus, 600);
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

onMounted(() => {
  fetchStatus();
});

onBeforeUnmount(() => {
  stopPolling();
});
</script>

<style scoped>
.live-runner-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.runner-header-card {
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

.status-indicator-box {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 20px;
  padding: 0.35rem 0.85rem;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
}

.dot-idle { background: #64748b; }
.dot-running {
  background: #3b82f6;
  box-shadow: 0 0 10px #3b82f6;
  animation: pulse 1.2s infinite;
}
.dot-completed { background: #10b981; }
.dot-error { background: #ef4444; }

@keyframes pulse {
  0% { transform: scale(0.95); opacity: 0.8; }
  50% { transform: scale(1.15); opacity: 1; }
  100% { transform: scale(0.95); opacity: 0.8; }
}

.status-text {
  font-size: 0.82rem;
  font-weight: 700;
  color: var(--text-heading);
}

/* Config Card */
.config-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 1.25rem;
}

.config-item {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.config-item label {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--text-muted);
}

.cfg-select {
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  color: var(--text-heading);
  padding: 0.55rem 0.85rem;
  border-radius: 8px;
  font-size: 0.88rem;
  outline: none;
}

.cfg-range {
  margin-top: 0.4rem;
}

.config-actions {
  display: flex;
  justify-content: flex-end;
}

.btn-start {
  padding: 0.75rem 1.75rem;
  font-size: 0.95rem;
  font-weight: 800;
}

.btn-stop {
  padding: 0.75rem 1.75rem;
  font-size: 0.95rem;
  font-weight: 800;
  background: var(--accent-rose);
  border: none;
  color: #fff;
  border-radius: 8px;
  cursor: pointer;
}

/* Monitor Card */
.monitor-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.monitor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.monitor-title h4 {
  color: var(--text-heading);
  font-size: 1.1rem;
  font-weight: 700;
}

.monitor-sub {
  color: var(--text-muted);
  font-size: 0.82rem;
}

.time-tag {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  padding: 0.2rem 0.6rem;
  border-radius: 6px;
  font-size: 0.78rem;
  font-family: var(--font-mono);
  color: var(--text-heading);
}

/* Progress */
.progress-section {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.progress-labels {
  display: flex;
  justify-content: space-between;
  font-size: 0.82rem;
  color: var(--text-muted);
  font-weight: 600;
}

.percent-lbl {
  font-family: var(--font-mono);
  font-weight: 800;
  color: var(--text-heading);
}

.progress-track {
  height: 10px;
  background: var(--bg-subtle);
  border-radius: 5px;
  overflow: hidden;
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.2);
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #6366f1 0%, #a855f7 35%, #38bdf8 70%, #6366f1 100%);
  background-size: 200% 100%;
  animation: barShimmer 2.2s linear infinite;
  transition: width 0.35s cubic-bezier(0.4, 0, 0.2, 1);
  border-radius: 5px;
  box-shadow: 0 0 12px rgba(99, 102, 241, 0.4);
}

@keyframes barShimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

.fill-completed {
  background: linear-gradient(90deg, #10b981, #059669);
  animation: none;
  box-shadow: 0 0 12px rgba(16, 185, 129, 0.4);
}

/* Live KPI */
.live-kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 1rem;
}

.live-kpi-card {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 0.85rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.kpi-lbl {
  font-size: 0.72rem;
  color: var(--text-muted);
  font-weight: 700;
}

.kpi-val {
  font-family: var(--font-mono);
  font-size: 1.4rem;
  font-weight: 900;
}

.kpi-sub {
  font-size: 0.72rem;
  color: var(--text-faint);
}

.text-emerald { color: #10b981; }
.text-amber { color: #f59e0b; }
.text-sky { color: #0ea5e9; }

/* Completion Banner */
.completion-banner {
  background: rgba(16, 185, 129, 0.12);
  border: 1px solid rgba(16, 185, 129, 0.35);
  border-radius: 10px;
  padding: 1.15rem 1.35rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}

.banner-left {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.banner-icon {
  font-size: 2rem;
}

.banner-left strong {
  color: #10b981;
  font-size: 0.95rem;
}

.banner-left p {
  color: var(--text-muted);
  font-size: 0.8rem;
  margin-top: 0.15rem;
  font-family: var(--font-mono);
}

.btn-success {
  background: #10b981;
  color: #ffffff;
  border: none;
  font-weight: 700;
  padding: 0.6rem 1.25rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-success:hover {
  background: #059669;
}

/* Table */
.live-cases-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.section-title h5 {
  font-size: 0.88rem;
  color: var(--text-heading);
}

.case-id-cell {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 0.82rem;
}
</style>
