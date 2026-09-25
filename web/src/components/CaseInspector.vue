<template>
  <div class="case-inspector-container">
    <!-- Filter Control Bar -->
    <div class="filter-card">
      <div class="filter-top-row">
        <!-- Model Selection -->
        <div class="filter-item">
          <label>当前评测模型:</label>
          <select v-model="selectedModelId" class="filter-select">
            <option v-for="m in models" :key="m.model_id" :value="m.model_id">
              {{ m.model_name || m.model_id }}
            </option>
          </select>
        </div>

        <!-- Status Filter -->
        <div class="filter-item">
          <label>评测状态:</label>
          <div class="btn-group">
            <button
              class="btn-filter"
              :class="{ active: filterStatus === 'all' }"
              @click="filterStatus = 'all'"
            >
              全部 ({{ allCases.length }})
            </button>
            <button
              class="btn-filter filter-fail"
              :class="{ active: filterStatus === 'failed' }"
              @click="filterStatus = 'failed'"
            >
              ❌ 坏例/未通过 ({{ failedCasesCount }})
            </button>
            <button
              class="btn-filter filter-pass"
              :class="{ active: filterStatus === 'passed' }"
              @click="filterStatus = 'passed'"
            >
              ✅ 已通过 ({{ passedCasesCount }})
            </button>
          </div>
        </div>

        <!-- Tier Filter -->
        <div class="filter-item">
          <label>难度分级:</label>
          <select v-model="filterTier" class="filter-select">
            <option value="all">全部阶梯 (L1 ~ L5)</option>
            <option value="frontier">⚡ 极限前沿 (L4 + L5)</option>
            <option value="L1">L1 基础自洽</option>
            <option value="L2">L2 标准任务</option>
            <option value="L3">L3 专业领域</option>
            <option value="L4">L4 自主智能体</option>
            <option value="L5">L5 极限难题</option>
          </select>
        </div>

        <!-- Category Filter -->
        <div class="filter-item">
          <label>任务分类:</label>
          <select v-model="filterCategory" class="filter-select">
            <option value="all">全部分类 (全部 18 类)</option>
            <option v-for="(name, catKey) in CATEGORY_NAMES" :key="catKey" :value="catKey">
              {{ name }}
            </option>
          </select>
        </div>
      </div>

      <div class="filter-bottom-row">
        <!-- Search Input -->
        <div class="search-input-wrap">
          <input
            v-model="searchQuery"
            type="text"
            placeholder="🔍 搜索测试用例 ID、错误原因、模型输出关健词..."
            class="case-search-input"
          />
          <button v-if="searchQuery" class="clear-btn" @click="searchQuery = ''">✕</button>
        </div>

        <div class="filter-actions">
          <button class="btn btn-secondary" @click="exportCurrentBadcases">
            📥 导出坏例清单 (CSV)
          </button>
        </div>
      </div>
    </div>

    <!-- Active Filter Summary Bar -->
    <div class="stats-summary-ribbon">
      <div class="summary-pill">
        <span class="pill-title">筛选命中用例:</span>
        <span class="pill-number">{{ filteredCases.length }}</span>
      </div>
      <div class="summary-pill">
        <span class="pill-title">当前子集通过率:</span>
        <span class="pill-number highlight">{{ subsetPassRate }}%</span>
      </div>
      <div class="summary-pill">
        <span class="pill-title">平均耗时:</span>
        <span class="pill-number">{{ subsetAvgLatency }} ms</span>
      </div>
      <div class="summary-pill">
        <span class="pill-title">平均 TPS:</span>
        <span class="pill-number">{{ subsetAvgTps }} tok/s</span>
      </div>
    </div>

    <!-- Cases List -->
    <div class="cases-list-wrapper">
      <div v-if="paginatedCases.length === 0" class="no-data-card">
        <div class="empty-icon">📂</div>
        <h4>未找到符合条件的测试用例</h4>
        <p>请尝试重置筛选器或修改搜索关键词</p>
      </div>

      <div
        v-for="c in paginatedCases"
        :key="c.test_case_id"
        class="case-card"
        :class="{ 'card-failed': !c.passed, 'card-expanded': isExpanded(c.test_case_id) }"
      >
        <!-- Case Card Top Bar -->
        <div class="case-header" @click="toggleExpand(c.test_case_id)">
          <div class="case-title-area">
            <span class="status-indicator" :class="c.passed ? 'status-pass' : 'status-fail'">
              {{ c.passed ? '✓ 通过' : '✕ 失败' }}
            </span>
            <span class="case-id">{{ c.test_case_id }}</span>
            <span class="badge" :class="'badge-tier-' + (c.resolved_tier || 'l3').toLowerCase()">
              {{ c.resolved_tier || 'L3' }}
            </span>
            <span class="badge badge-info">
              {{ CATEGORY_NAMES[c.category] || c.category }}
            </span>
          </div>

          <div class="case-metrics-preview">
            <div class="metric-chip">
              <span class="lbl">得分:</span>
              <span class="val" :class="c.score >= 0.8 ? 'good' : 'bad'">{{ (c.score || 0).toFixed(2) }}</span>
            </div>
            <div class="metric-chip">
              <span class="lbl">延迟:</span>
              <span class="val">{{ Math.round(c.latency_ms || 0) }} ms</span>
            </div>
            <div class="metric-chip" v-if="c.tps">
              <span class="lbl">TPS:</span>
              <span class="val">{{ c.tps.toFixed(1) }}</span>
            </div>
            <button class="expand-icon-btn">
              {{ isExpanded(c.test_case_id) ? '▲ 收起' : '▼ 展开详情' }}
            </button>
          </div>
        </div>

        <!-- 5-Dimensional Metrics Bar -->
        <div v-if="c.dimensions" class="dimensions-strip">
          <div class="dim-tag" title="任务目标达成率">
            <span class="dim-name">🎯 目标达成</span>
            <span class="dim-val">{{ (c.dimensions.goal_score ?? 1).toFixed(2) }}</span>
          </div>
          <div class="dim-tag" title="工具调用规范与参数匹配度">
            <span class="dim-name">🛠️ 工具规范</span>
            <span class="dim-val">{{ (c.dimensions.tool_score ?? 1).toFixed(2) }}</span>
          </div>
          <div class="dim-tag" title="思维链推理深度与自洽性">
            <span class="dim-name">🧠 推理逻辑</span>
            <span class="dim-val">{{ (c.dimensions.reasoning_score ?? 1).toFixed(2) }}</span>
          </div>
          <div class="dim-tag" title="环境报错自愈与排障回退">
            <span class="dim-name">🔄 异常自愈</span>
            <span class="dim-val">{{ (c.dimensions.recovery_score ?? 1).toFixed(2) }}</span>
          </div>
          <div class="dim-tag" title="轮次与Token经济性">
            <span class="dim-name">⚡ 执行能效</span>
            <span class="dim-val">{{ (c.dimensions.efficiency_score ?? 1).toFixed(2) }}</span>
          </div>
        </div>

        <!-- Expanded Details Area -->
        <div v-if="isExpanded(c.test_case_id)" class="case-expanded-body">
          <!-- Multi-Turn Step-by-Step Trajectory Timeline (if present) -->
          <div v-if="c.trajectory_steps && c.trajectory_steps.length > 0" class="detail-section trajectory-section">
            <div class="section-title trajectory-title">
              <span class="icon">🔄</span>
              <strong>多轮智能体交互与工具调用时间线 (Multi-Turn ReAct Timeline)</strong>
              <span class="token-hint">共 {{ c.trajectory_steps.length }} 轮次交互</span>
            </div>
            <div class="trajectory-timeline">
              <div
                v-for="step in c.trajectory_steps"
                :key="step.turn"
                class="timeline-step-item"
              >
                <div class="step-badge-col">
                  <span class="step-num-badge">第 {{ step.turn }} 轮</span>
                  <span class="step-lat-badge">{{ step.latency_ms }}ms</span>
                </div>
                <div class="step-content-card">
                  <div v-if="step.model_thought" class="step-thought-text">
                    <span class="thought-tag">💭 模型思考:</span> {{ step.model_thought }}
                  </div>
                  <div v-if="step.tool_calls && step.tool_calls.length > 0" class="step-tools-box">
                    <div v-for="(tc, tcIdx) in step.tool_calls" :key="tcIdx" class="tool-call-row">
                      <span class="tool-name-badge">🛠️ {{ tc.function?.name || tc.name || 'tool' }}</span>
                      <code class="tool-args-code">{{ typeof tc.function?.arguments === 'string' ? tc.function.arguments : JSON.stringify(tc.function?.arguments || tc.arguments || {}) }}</code>
                    </div>
                  </div>
                  <div v-if="step.tool_results && step.tool_results.length > 0" class="step-results-box">
                    <div v-for="(res, resIdx) in step.tool_results" :key="resIdx" class="tool-result-row">
                      <span class="result-lbl">📤 环境变量/执行反馈:</span>
                      <pre class="tool-result-pre">{{ res[1] || res }}</pre>
                    </div>
                  </div>
                  <div v-if="step.error" class="step-error-box">
                    ⚠️ {{ step.error }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Deep Thinking Trace Section (if present) -->
          <div v-if="getThinking(c).thinking" class="detail-section thinking-section">
            <div class="section-title thinking-title">
              <span class="icon">🧠</span>
              <strong>深度思考链 (Deep Thinking Trace)</strong>
              <span class="token-hint">{{ getThinking(c).thinking.length }} 字符</span>
            </div>
            <div class="code-box thinking-box">{{ getThinking(c).thinking }}</div>
          </div>

          <!-- Final Model Output Section -->
          <div class="detail-section">
            <div class="section-title">
              <span class="icon">💬</span>
              <strong>模型最终输出 (Model Output)</strong>
            </div>
            <div class="code-box output-box">{{ getThinking(c).response || '（无输出内容）' }}</div>
          </div>

          <!-- Verification Reason / Execution Trace -->
          <div class="detail-section">
            <div class="section-title">
              <span class="icon">🧪</span>
              <strong>判定结论与测试运行反馈 (Verifier Output)</strong>
            </div>
            <div class="code-box verifier-box">{{ c.reason || '（无测试反馈）' }}</div>
          </div>

          <!-- Error Details if failed -->
          <div v-if="c.error" class="detail-section error-section">
            <div class="section-title error-title">
              <span class="icon">⚠️</span>
              <strong>运行时异常与错误堆栈 (Execution Error)</strong>
            </div>
            <div class="code-box error-box">{{ c.error }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Pagination Controls -->
    <div v-if="totalPages > 1" class="pagination-bar">
      <div class="page-info">
        显示第 {{ (currentPage - 1) * pageSize + 1 }} 至 {{ Math.min(currentPage * pageSize, filteredCases.length) }} 条，共 {{ filteredCases.length }} 条
      </div>
      <div class="pagination-buttons">
        <button
          class="btn-page"
          :disabled="currentPage === 1"
          @click="currentPage = 1"
        >
          ⏮ 首页
        </button>
        <button
          class="btn-page"
          :disabled="currentPage === 1"
          @click="currentPage--"
        >
          ◀ 上一页
        </button>
        <span class="current-page-tag">{{ currentPage }} / {{ totalPages }}</span>
        <button
          class="btn-page"
          :disabled="currentPage === totalPages"
          @click="currentPage++"
        >
          下一页 ▶
        </button>
        <button
          class="btn-page"
          :disabled="currentPage === totalPages"
          @click="currentPage = totalPages"
        >
          末页 ⏭
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue';
import { CATEGORY_NAMES, resolveDifficulty, extractThinking } from '../utils/benchmark';
import { exportCasesCsv } from '../utils/export';

const props = defineProps({
  models: {
    type: Array,
    required: true
  },
  initialModelId: {
    type: String,
    default: ''
  }
});

const selectedModelId = ref(props.initialModelId || (props.models[0]?.model_id || ''));

watch(() => props.initialModelId, (newId) => {
  if (newId) selectedModelId.value = newId;
});

watch(() => props.models, (newModels) => {
  if (newModels.length > 0 && (!selectedModelId.value || !newModels.find(m => m.model_id === selectedModelId.value))) {
    selectedModelId.value = newModels[0].model_id;
  }
});

// Filters
const filterStatus = ref('all'); // 'all', 'failed', 'passed'
const filterTier = ref('all'); // 'all', 'frontier', 'L1', 'L2', 'L3', 'L4', 'L5'
const filterCategory = ref('all');
const searchQuery = ref('');

// Pagination
const currentPage = ref(1);
const pageSize = ref(20);

// Expanded case IDs set
const expandedCases = ref(new Set());

function isExpanded(id) {
  return expandedCases.value.has(id);
}

function toggleExpand(id) {
  if (expandedCases.value.has(id)) {
    expandedCases.value.delete(id);
  } else {
    expandedCases.value.add(id);
  }
}

// Current active model summary
const currentModel = computed(() => {
  return props.models.find(m => m.model_id === selectedModelId.value) || props.models[0];
});

// Raw cases
const allCases = computed(() => {
  return currentModel.value?.case_results || [];
});

const failedCasesCount = computed(() => {
  return allCases.value.filter(c => !c.passed).length;
});

const passedCasesCount = computed(() => {
  return allCases.value.filter(c => c.passed).length;
});

// Filtered cases
const filteredCases = computed(() => {
  const query = searchQuery.value.trim().toLowerCase();

  return allCases.value.filter(c => {
    // Status
    if (filterStatus.value === 'failed' && c.passed) return false;
    if (filterStatus.value === 'passed' && !c.passed) return false;

    // Tier
    const tier = c.resolved_tier || resolveDifficulty(c);
    if (filterTier.value === 'frontier') {
      if (tier !== 'L4' && tier !== 'L5') return false;
    } else if (filterTier.value !== 'all') {
      if (tier !== filterTier.value) return false;
    }

    // Category
    if (filterCategory.value !== 'all' && c.category !== filterCategory.value) {
      return false;
    }

    // Search query
    if (query) {
      const matchId = (c.test_case_id || '').toLowerCase().includes(query);
      const matchReason = (c.reason || '').toLowerCase().includes(query);
      const matchOutput = (c.model_output || '').toLowerCase().includes(query);
      const matchError = (c.error || '').toLowerCase().includes(query);
      if (!matchId && !matchReason && !matchOutput && !matchError) return false;
    }

    return true;
  });
});

// Reset page when filter changes
watch([selectedModelId, filterStatus, filterTier, filterCategory, searchQuery], () => {
  currentPage.value = 1;
});

const totalPages = computed(() => {
  return Math.ceil(filteredCases.value.length / pageSize.value) || 1;
});

const paginatedCases = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value;
  return filteredCases.value.slice(start, start + pageSize.value);
});

// Subset statistics
const subsetPassRate = computed(() => {
  if (filteredCases.value.length === 0) return 0;
  const p = filteredCases.value.filter(c => c.passed).length;
  return ((p / filteredCases.value.length) * 100).toFixed(1);
});

const subsetAvgLatency = computed(() => {
  if (filteredCases.value.length === 0) return 0;
  const sum = filteredCases.value.reduce((acc, c) => acc + (c.latency_ms || 0), 0);
  return Math.round(sum / filteredCases.value.length);
});

const subsetAvgTps = computed(() => {
  const valid = filteredCases.value.filter(c => c.tps > 0);
  if (valid.length === 0) return 0;
  const sum = valid.reduce((acc, c) => acc + c.tps, 0);
  return (sum / valid.length).toFixed(1);
});

function getThinking(c) {
  return extractThinking(c.model_output);
}

function exportCurrentBadcases() {
  const badcases = filteredCases.value.filter(c => !c.passed);
  if (badcases.length === 0) {
    alert('当前筛选条件下没有坏例！');
    return;
  }
  const filename = `badcases_${selectedModelId.value}_${Date.now()}.csv`;
  exportCasesCsv(badcases, filename);
}
</script>

<style scoped>
.case-inspector-container {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.filter-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.filter-top-row {
  display: flex;
  flex-wrap: wrap;
  gap: 1.25rem;
  align-items: center;
}

.filter-item {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.filter-item label {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-weight: 600;
}

.filter-select {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  color: var(--text-heading);
  padding: 0.45rem 0.8rem;
  border-radius: 8px;
  font-size: 0.85rem;
  font-family: var(--font-sans);
  outline: none;
  cursor: pointer;
}

.filter-select:focus {
  border-color: var(--primary);
}

.btn-group {
  display: flex;
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  overflow: hidden;
}

.btn-filter {
  background: transparent;
  border: none;
  color: var(--text-muted);
  padding: 0.45rem 0.85rem;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-filter:hover {
  background: var(--bg-hover);
  color: var(--text-heading);
}

.btn-filter.active {
  background: var(--primary);
  color: #fff;
}

.btn-filter.filter-fail.active {
  background: var(--accent-rose);
  color: #fff;
}

.btn-filter.filter-pass.active {
  background: var(--accent-emerald);
  color: #fff;
}

.filter-bottom-row {
  display: flex;
  gap: 1rem;
  align-items: center;
}

.search-input-wrap {
  flex: 1;
  position: relative;
}

.case-search-input {
  width: 100%;
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  color: var(--text-heading);
  padding: 0.55rem 2.2rem 0.55rem 0.85rem;
  border-radius: 8px;
  font-size: 0.85rem;
  outline: none;
}

.case-search-input:focus {
  border-color: var(--primary);
}

.clear-btn {
  position: absolute;
  right: 0.6rem;
  top: 50%;
  transform: translateY(-50%);
  background: transparent;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  font-size: 0.9rem;
}

.btn {
  padding: 0.55rem 1rem;
  border-radius: 8px;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-secondary {
  background: var(--bg-subtle);
  border: 1px solid var(--border-soft);
  color: var(--text-body);
}

.btn-secondary:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

/* Ribbon */
.stats-summary-ribbon {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 0.75rem 1.25rem;
}

.summary-pill {
  display: flex;
  align-items: baseline;
  gap: 0.4rem;
  font-size: 0.82rem;
}

.pill-title {
  color: var(--text-muted);
}

.pill-number {
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--text-heading);
}

.pill-number.highlight {
  color: var(--primary);
  font-size: 1rem;
}

/* Case List */
.cases-list-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.no-data-card {
  background: var(--bg-card);
  border: 1px dashed var(--border-strong);
  border-radius: 12px;
  padding: 3rem;
  text-align: center;
  color: var(--text-muted);
}

.empty-icon {
  font-size: 2.5rem;
  margin-bottom: 0.5rem;
}

.case-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  overflow: hidden;
  transition: border-color 0.2s ease;
}

.case-card:hover {
  border-color: var(--border-strong);
}

.card-failed {
  border-left: 4px solid var(--accent-rose);
}

.case-header {
  padding: 0.85rem 1.25rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  background: var(--bg-card);
}

.case-title-area {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.status-indicator {
  font-size: 0.75rem;
  font-weight: 800;
  padding: 0.2rem 0.5rem;
  border-radius: 6px;
}

.status-pass {
  background: var(--emerald-bg);
  color: var(--accent-emerald);
  border: 1px solid var(--emerald-border);
}

.status-fail {
  background: var(--rose-bg);
  color: var(--accent-rose);
  border: 1px solid var(--rose-border);
}

.case-id {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 0.9rem;
  color: var(--text-heading);
}

.case-metrics-preview {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.metric-chip {
  display: flex;
  align-items: baseline;
  gap: 0.25rem;
  font-size: 0.78rem;
}

.metric-chip .lbl {
  color: var(--text-faint);
}

.metric-chip .val {
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--text-body);
}

.metric-chip .val.good { color: var(--accent-emerald); }
.metric-chip .val.bad { color: var(--accent-rose); }

.expand-icon-btn {
  background: transparent;
  border: none;
  color: var(--primary);
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
}

.dimensions-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding: 0.45rem 1.25rem;
  background: var(--bg-surface);
  border-top: 1px solid var(--border-soft);
}

.dim-tag {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.72rem;
  background: var(--bg-subtle);
  border: 1px solid var(--border-soft);
  padding: 0.15rem 0.5rem;
  border-radius: 4px;
}

.dim-name {
  color: var(--text-muted);
}

.dim-val {
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--text-heading);
}

.case-expanded-body {
  padding: 1.25rem;
  background: var(--bg-surface);
  border-top: 1px solid var(--border-soft);
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.detail-section {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.82rem;
  color: var(--text-heading);
}

.thinking-title {
  color: #c084fc;
}

.token-hint {
  font-size: 0.72rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
  margin-left: auto;
}

.thinking-box {
  border-color: rgba(168, 85, 247, 0.35);
  background: rgba(22, 16, 38, 0.6);
  color: #e9d5ff;
}

.output-box {
  color: #e2e8f0;
}

.verifier-box {
  background: rgba(15, 23, 42, 0.7);
  border-color: rgba(255, 255, 255, 0.08);
}

.error-title {
  color: var(--accent-rose);
}

.error-box {
  background: rgba(69, 10, 10, 0.4);
  border-color: var(--rose-border);
  color: #fca5a5;
}

/* Pagination */
.pagination-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem;
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.page-info {
  font-size: 0.8rem;
  color: var(--text-muted);
}

.pagination-buttons {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.btn-page {
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  color: var(--text-body);
  padding: 0.35rem 0.75rem;
  border-radius: 6px;
  font-size: 0.78rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-page:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-heading);
}

.btn-page:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.current-page-tag {
  font-size: 0.8rem;
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--text-heading);
  padding: 0 0.5rem;
}

/* Trajectory Timeline Styles */
.trajectory-section {
  background: var(--bg-card);
  border: 1px solid rgba(99, 102, 241, 0.25);
  border-radius: 8px;
  padding: 0.85rem;
}

.trajectory-title {
  color: #818cf8;
}

.trajectory-timeline {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  margin-top: 0.5rem;
}

.timeline-step-item {
  display: flex;
  gap: 0.85rem;
  position: relative;
}

.step-badge-col {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
  min-width: 68px;
}

.step-num-badge {
  background: #6366f1;
  color: #ffffff;
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.2rem 0.45rem;
  border-radius: 4px;
}

.step-lat-badge {
  font-size: 0.68rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
}

.step-content-card {
  flex: 1;
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 0.65rem 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.step-thought-text {
  font-size: 0.8rem;
  color: #e2e8f0;
  line-height: 1.45;
}

.thought-tag {
  color: #c084fc;
  font-weight: 700;
}

.step-tools-box {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.tool-call-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: var(--bg-card);
  padding: 0.3rem 0.55rem;
  border-radius: 4px;
  border: 1px solid var(--border-soft);
  font-size: 0.78rem;
}

.tool-name-badge {
  color: #38bdf8;
  font-weight: 700;
  white-space: nowrap;
}

.tool-args-code {
  font-family: var(--font-mono);
  color: #a5f3fc;
  font-size: 0.72rem;
  overflow-x: auto;
}

.step-results-box {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.tool-result-row {
  background: rgba(15, 23, 42, 0.8);
  border: 1px solid var(--border-soft);
  border-radius: 4px;
  padding: 0.4rem 0.6rem;
}

.result-lbl {
  display: block;
  font-size: 0.7rem;
  color: #94a3b8;
  font-weight: 600;
  margin-bottom: 0.15rem;
}

.tool-result-pre {
  font-family: var(--font-mono);
  font-size: 0.72rem;
  color: #94a3b8;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 120px;
  overflow-y: auto;
}

.step-error-box {
  background: rgba(239, 68, 68, 0.15);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #f87171;
  padding: 0.35rem 0.6rem;
  border-radius: 4px;
  font-size: 0.75rem;
}
</style>
