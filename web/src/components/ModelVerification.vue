<template>
  <div class="model-verification-container">
    <!-- Header Card -->
    <div class="verify-header-card">
      <div class="header-left">
        <h3 class="panel-title">🛡️ 模型验真、探针嗅探与基线库（非官方认证）</h3>
        <p class="panel-subtitle">
          使用 <strong>6 大静态探针</strong>、<strong>确定性动态对抗题 + 可选 LLM 独立审计</strong> 与 <strong>标注“已核实/未核实”的基准基线库</strong>。探针结果不是密码学证明，未运行 benchmark 时不会生成基线漂移。
        </p>
      </div>

      <!-- Mode Selector Tabs -->
      <div class="tab-switcher">
        <button
          class="tab-btn"
          :class="{ active: activeMode === 'agent' }"
          @click="activeMode = 'agent'"
        >
          🤖 动态对抗审计（可选 LLM 审计）
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeMode === 'probe' }"
          @click="activeMode = 'probe'"
        >
          ⚡ 快速高敏探针嗅探
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeMode === 'baselines' }"
          @click="activeMode = 'baselines'"
        >
          🏛️ 服务端基线库
        </button>
      </div>
    </div>

    <!-- ========================================================================= -->
    <!-- TAB 1: 🤖 LLM-Driven Autonomous Verification Agent                        -->
    <!-- ========================================================================= -->
    <div v-if="activeMode === 'agent'" class="mode-content-block fade-in">
      <!-- Control Bar -->
      <div class="config-card">
        <div class="config-grid">
          <div class="config-item">
            <label>测试端点模型:</label>
            <select v-model="formModel" :disabled="isAgentAuditing" class="cfg-select">
              <option v-for="m in models" :key="m.model_id" :value="m.model_id">
                {{ m.model_name || m.model_id }}
              </option>
              <option value="mock-pro">🤖 Mock-Pro-v1 (模拟深度推理旗舰)</option>
              <option value="mock-fast">⚡ Mock-Fast-v1 (模拟轻量极速模型)</option>
            </select>
          </div>

          <div class="config-item">
            <label>标称声称模型 (Target Claim):</label>
            <select v-model="formTarget" :disabled="isAgentAuditing" class="cfg-select">
              <option v-for="opt in claimedTargets" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
          </div>

          <div class="config-item">
            <label>自定义端点 URL (需服务端 --allow-custom-endpoints):</label>
            <input
              v-model="formBaseUrl"
              type="text"
              placeholder="留空则复用模型默认配置或 Mock"
              :disabled="isAgentAuditing"
              class="cfg-input"
            />
          </div>

          <div class="config-item">
            <label>自定义 API 密钥 (需服务端显式开启):</label>
            <input
              v-model="formApiKey"
              type="password"
              placeholder="sk-..."
              :disabled="isAgentAuditing"
              class="cfg-input"
            />
          </div>
        </div>

        <div class="config-actions">
          <button
            class="btn btn-primary btn-start-agent"
            :disabled="isAgentAuditing"
            @click="startAgentAudit"
          >
            <span v-if="!isAgentAuditing">🤖 启动动态对抗审计 (Launch Audit)</span>
            <span v-else class="scanning-text">
              <span class="spinner-sm"></span> 正在现场合成动态对抗题目并执行交叉审讯...
            </span>
          </button>
        </div>
      </div>

      <div v-if="agentAuditError" class="error-banner fade-in">⚠️ 验真请求失败：{{ agentAuditError }}</div>

      <!-- Agent Scanning Radar / Terminal Animation -->
      <div v-if="isAgentAuditing" class="scanning-card fade-in">
        <div class="radar-scan-box agent-radar">
          <div class="radar-ring r1"></div>
          <div class="radar-ring r2"></div>
          <div class="radar-ring r3"></div>
          <div class="radar-sweep agent-sweep"></div>
          <div class="radar-center-icon">🤖</div>
        </div>
        <div class="scanning-info">
          <h4>动态对抗审计正在展开多轮审讯...</h4>
          <p>
            第 1 阶段：现场动态合成随机代数双约束陷阱与分词穿透题 (防范静态正则缓存)<br />
            第 2 阶段：针对被测模型输出抛出伪造专家异议，执行极限施压以侦测【阿谀盲从/谄媚倾向 (Sycophancy)】<br />
            第 3 阶段：只做行为证据归纳；未生成 verified 基线时不生成“实测基线分”
          </p>
        </div>
      </div>

      <!-- Agent Audit Report -->
      <div v-if="agentAuditReport && !isAgentAuditing" class="report-wrapper fade-in">
        <!-- Certificate Ribbon Card -->
        <div
          class="certificate-card"
          :class="'cert-' + agentAuditReport.verdict_grade.toLowerCase()"
        >
          <div class="cert-score-col">
            <div class="score-dial">
              <span class="dial-num">{{ (agentAuditReport.evidence_score ?? agentAuditReport.authenticity_score).toFixed(1) }}</span>
              <span class="dial-unit">证据分</span>
            </div>
          </div>

          <div class="cert-main-col">
            <div class="cert-title-row">
              <span class="cert-verdict-title">{{ agentAuditReport.verdict_title }}</span>
              <span class="cert-time">审计时间: {{ agentAuditReport.timestamp }}</span>
            </div>

            <p class="cert-summary">{{ agentAuditReport.executive_summary }}</p>
            <div v-if="agentAuditReport.signals && agentAuditReport.signals.length" class="signal-matrix">
              <h5>动态证据矩阵（非认证）</h5>
              <div v-for="s in agentAuditReport.signals" :key="s.id" class="signal-row" :class="'signal-' + s.status">
                <span class="signal-name">{{ s.label }}</span>
                <span class="signal-status">{{ signalStatus(s.status) }}</span>
                <span class="signal-detail">{{ s.detail }}</span>
              </div>
            </div>
            <p v-if="!agentAuditReport.baseline_drift" class="config-hint">
              本次未生成 verified 基线漂移；固定/动态探针与公开 benchmark 小样本是证据，不是正品认证，也不能替代官方全量分数。
            </p>

            <div class="cert-meta-tags">
              <div class="meta-tag">
                <span class="tag-lbl">测试端点:</span>
                <span class="tag-val font-mono">{{ agentAuditReport.target_model }}</span>
              </div>
              <div class="meta-tag">
                <span class="tag-lbl">参考基线:</span>
                <span class="tag-val font-mono text-gold">{{ agentAuditReport.canonical_baseline_name }}</span>
              </div>
              <div class="meta-tag">
                <span class="tag-lbl">阿谀附和指数:</span>
                <span class="tag-val font-mono" :class="agentAuditReport.sycophancy_index > 0 ? 'text-rose' : 'text-emerald'">
                  {{ agentAuditReport.sycophancy_index.toFixed(1) }}% ({{ agentAuditReport.sycophancy_assessment }})
                </span>
              </div>
              <div class="meta-tag">
                <span class="tag-lbl">审讯总耗时:</span>
                <span class="tag-val font-mono">{{ agentAuditReport.total_latency_ms }} ms</span>
              </div>
            </div>

            <!-- Evidence Warning Tags -->
            <div v-if="agentAuditReport.audit_evidence_chain && agentAuditReport.audit_evidence_chain.length > 0" class="risk-tags-row">
              <span class="risk-label">审计告警存证:</span>
              <span
                v-for="(ev, idx) in agentAuditReport.audit_evidence_chain"
                :key="idx"
                class="risk-chip text-rose"
              >
                ⚠️ {{ ev }}
              </span>
            </div>
          </div>

          <div class="cert-action-col">
            <button class="btn btn-secondary btn-copy-report" @click="copyAgentReportMarkdown">
              {{ isCopied ? '✓ 已复制完整凭证' : '📋 复制审计报告' }}
            </button>
          </div>
        </div>

        <!-- Dynamic Probes Table -->
        <div class="probes-table-card">
          <div class="table-card-header">
            <h4>🎯 现场动态合成对抗探针执行结果 (Dynamic Adversarial Probes)</h4>
            <span class="header-hint">现场随机代数与离散词汇合成，有效粉碎中转代理正则重定向拦截</span>
          </div>

          <div class="probes-table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th style="width: 25%;">对抗题目类型</th>
                  <th style="width: 15%;">陷阱机制</th>
                  <th style="width: 10%;">判定状态</th>
                  <th style="width: 10%;">CoT思考链</th>
                  <th style="width: 10%;">耗时</th>
                  <th style="width: 30%;">Agent 审计点评</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="dp in agentAuditReport.dynamic_probes" :key="dp.id">
                  <td>
                    <div class="probe-title-cell">
                      <span class="probe-title">{{ dp.title }}</span>
                      <span class="probe-id-sub">{{ dp.id }}</span>
                    </div>
                  </td>
                  <td>
                    <span class="badge badge-info">{{ dp.trap_type }}</span>
                  </td>
                  <td>
                    <span class="badge" :class="dp.passed ? 'badge-success' : 'badge-danger'">
                      {{ dp.passed ? '✓ 通过' : '✕ 踩雷' }}
                    </span>
                  </td>
                  <td>
                    <span class="badge" :class="dp.has_cot_signature ? 'badge-info' : 'badge-neutral'">
                      {{ dp.has_cot_signature ? '✓ 具备' : '-' }}
                    </span>
                  </td>
                  <td class="font-mono">{{ dp.response_latency_ms }} ms</td>
                  <td>
                    <div class="findings-cell" :class="dp.passed ? 'text-emerald' : 'text-rose'">
                      {{ dp.auditor_critique }}
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Cross-Examination Dialogues Card -->
        <div v-if="agentAuditReport.cross_examinations && agentAuditReport.cross_examinations.length > 0" class="interrogation-card">
          <div class="table-card-header">
            <h4>💬 多轮交叉审讯式追问与抗压记录 (Cross-Examination Interrogation)</h4>
            <span class="header-hint">审讯员故意抛出伪造权威错误诱导，侦测模型是坚守严谨逻辑还是盲从道歉认错</span>
          </div>

          <div class="dialogue-list">
            <div
              v-for="(ce, idx) in agentAuditReport.cross_examinations"
              :key="idx"
              class="dialogue-item"
              :class="{ 'dialogue-warn': ce.sycophancy_detected }"
            >
              <div class="dialogue-header">
                <span class="dialogue-step-badge">第 {{ idx + 1 }} 轮交叉审讯</span>
                <span class="dialogue-probe-title">{{ ce.probe_title }}</span>
                <span
                  class="badge"
                  :class="ce.sycophancy_detected ? 'badge-danger' : 'badge-success'"
                >
                  {{ ce.sycophancy_detected ? '🚨 判定: 盲目阿谀妥协' : '🛡️ 判定: 坚持真理自洽' }}
                </span>
              </div>

              <div class="chat-thread">
                <!-- Interrogator Challenge Bubble -->
                <div class="bubble-row bubble-auditor">
                  <div class="bubble-avatar">🕵️‍♂️ 审讯 Agent</div>
                  <div class="bubble-content bubble-auditor-content">
                    {{ ce.interrogator_challenge }}
                  </div>
                </div>

                <!-- Target Model Defense Bubble -->
                <div class="bubble-row bubble-target">
                  <div class="bubble-avatar">🤖 被测模型</div>
                  <div class="bubble-content bubble-target-content">
                    {{ ce.target_defense_response }}
                  </div>
                </div>
              </div>

              <div class="dialogue-verdict-bar" :class="ce.sycophancy_detected ? 'bar-danger' : 'bar-success'">
                <span class="verdict-icon">{{ ce.sycophancy_detected ? '⚠️' : '✓' }}</span>
                <span class="verdict-text">{{ ce.auditor_notes }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Baseline Drift Card -->
        <div v-if="agentAuditReport.baseline_drift && agentAuditReport.baseline_drift.baseline_found" class="drift-table-card">
          <div class="table-card-header">
            <h4>🏛️ 服务端 verified 基线偏离度分析（未核实条目不参与）</h4>
            <span class="header-hint">与 {{ agentAuditReport.baseline_drift.vendor }} 原厂公布论文与技术报告指标对标</span>
          </div>

          <div class="drift-summary-ribbon">
            <div class="ribbon-item">
              <span class="r-lbl">官方论文出处:</span>
              <a :href="agentAuditReport.baseline_drift.tech_report_url" target="_blank" class="r-val text-sky">
                {{ agentAuditReport.baseline_drift.tech_report_url }} ↗
              </a>
            </div>
            <div class="ribbon-item">
              <span class="r-lbl">实测综合漂移 (Δ):</span>
              <span class="r-val font-mono" :class="agentAuditReport.baseline_drift.mean_drift_pct < -15 ? 'text-rose' : 'text-emerald'">
                {{ agentAuditReport.baseline_drift.mean_drift_pct > 0 ? '+' : '' }}{{ agentAuditReport.baseline_drift.mean_drift_pct.toFixed(1) }}%
              </span>
            </div>
            <div class="ribbon-item">
              <span class="r-lbl">吻合裁决:</span>
              <span class="r-val font-bold">{{ agentAuditReport.baseline_drift.verdict }}</span>
            </div>
          </div>

          <div class="probes-table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>权威基准项目</th>
                  <th>评测领域</th>
                  <th>官方技术报告公布分</th>
                  <th>实测得分</th>
                  <th>偏离度 (Δ)</th>
                  <th>状态判定</th>
                  <th>诊断解读</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="c in agentAuditReport.baseline_drift.comparisons" :key="c.benchmark_id">
                  <td class="font-bold text-sky">{{ c.benchmark_name }}</td>
                  <td>{{ c.category }}</td>
                  <td class="font-mono">{{ c.official_score.toFixed(1) }} {{ c.unit }}</td>
                  <td class="font-mono">{{ c.tested_score.toFixed(1) }} {{ c.unit }}</td>
                  <td class="font-mono" :class="c.delta_pct < -15 ? 'text-rose font-bold' : (c.delta_pct < -5 ? 'text-gold' : 'text-emerald')">
                    {{ c.delta_pct > 0 ? '+' : '' }}{{ c.delta_pct.toFixed(1) }}%
                  </td>
                  <td>
                    <span
                      class="badge"
                      :class="{
                        'badge-success': c.status === 'Matching',
                        'badge-warning': c.status === 'MinorDeficit',
                        'badge-danger': c.status === 'SevereDeficit',
                        'badge-info': c.status === 'SuspiciousOverfit'
                      }"
                    >
                      {{ getDriftBadgeLabel(c.status) }}
                    </span>
                  </td>
                  <td class="text-subtle">{{ c.diagnosis }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <!-- ========================================================================= -->
    <!-- TAB 2: ⚡ Fast Static Probe Fingerprinting (Original Enhanced)            -->
    <!-- ========================================================================= -->
    <div v-if="activeMode === 'probe'" class="mode-content-block fade-in">
      <!-- Config Form -->
      <div class="config-card">
        <div class="config-grid">
          <div class="config-item">
            <label>测试端点模型:</label>
            <select v-model="formModel" :disabled="isVerifying" class="cfg-select">
              <option v-for="m in models" :key="m.model_id" :value="m.model_id">
                {{ m.model_name || m.model_id }}
              </option>
              <option value="mock-pro">🤖 Mock-Pro-v1 (模拟深度思考模型)</option>
              <option value="mock-fast">⚡ Mock-Fast-v1 (模拟轻量极速模型)</option>
            </select>
          </div>

          <div class="config-item">
            <label>标称声称模型 (Target Claim):</label>
            <select v-model="formTarget" :disabled="isVerifying" class="cfg-select">
              <option v-for="opt in claimedTargets" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
          </div>

          <div class="config-item">
            <label>自定义端点 URL (需服务端 --allow-custom-endpoints):</label>
            <input
              v-model="formBaseUrl"
              type="text"
              placeholder="留空则复用模型默认配置或 Mock"
              :disabled="isVerifying"
              class="cfg-input"
            />
          </div>

          <div class="config-item">
            <label>自定义 API 密钥 (需服务端显式开启):</label>
            <input
              v-model="formApiKey"
              type="password"
              placeholder="sk-..."
              :disabled="isVerifying"
              class="cfg-input"
            />
          </div>
        </div>

        <div class="config-actions">
          <button
            class="btn btn-primary btn-start-verify"
            :disabled="isVerifying"
            @click="startVerification"
          >
            <span v-if="!isVerifying">🔍 启动快速高敏探针嗅探 (Run Fast Probes)</span>
            <span v-else class="scanning-text">
              <span class="spinner-sm"></span> 正在并发注入 6 大旗舰探针并分析指纹...
            </span>
          </button>
        </div>
      </div>

      <!-- Scanning Radar Animation -->
      <div v-if="isVerifying" class="scanning-card fade-in">
        <div class="radar-scan-box">
          <div class="radar-ring r1"></div>
          <div class="radar-ring r2"></div>
          <div class="radar-ring r3"></div>
          <div class="radar-sweep"></div>
          <div class="radar-center-icon">🛡️</div>
        </div>
        <div class="scanning-info">
          <h4>正在探测 【{{ formTarget }}】 原厂指纹与能力断崖...</h4>
          <p>正在执行：创作者身份声明、Strawberry 字符计数、Bat & Ball 直觉抑制、浮点量化敏感截断探测</p>
        </div>
      </div>

      <!-- Verification Certificate Report -->
      <div v-if="verificationReport && !isVerifying" class="report-wrapper fade-in">
        <div
          class="certificate-card"
          :class="'cert-' + verificationReport.verdict_grade.toLowerCase()"
        >
          <div class="cert-score-col">
            <div class="score-dial">
              <span class="dial-num">{{ (verificationReport.evidence_score ?? verificationReport.authenticity_score).toFixed(1) }}</span>
              <span class="dial-unit">证据分</span>
            </div>
          </div>

          <div class="cert-main-col">
            <div class="cert-title-row">
              <span class="cert-verdict-title">{{ verificationReport.verdict_title }}</span>
              <span class="cert-time">核验时间: {{ verificationReport.verified_at }}</span>
            </div>

            <p class="cert-summary">{{ verificationReport.summary }}</p>
            <div v-if="verificationReport.signals && verificationReport.signals.length" class="signal-matrix">
              <h5>证据矩阵（非认证）</h5>
              <div v-for="s in verificationReport.signals" :key="s.id" class="signal-row" :class="'signal-' + s.status">
                <span class="signal-name">{{ s.label }}</span>
                <span class="signal-status">{{ signalStatus(s.status) }}</span>
                <span class="signal-detail">{{ s.detail }}</span>
              </div>
            </div>
            <p v-if="!verificationReport.baseline_drift" class="config-hint">
              本次未生成 verified 基线漂移；固定探针与公开 benchmark 小样本是能力/行为证据，不是身份证明。
            </p>

            <div class="cert-meta-tags">
              <div class="meta-tag">
                <span class="tag-lbl">测试端点:</span>
                <span class="tag-val font-mono">{{ verificationReport.model_id }}</span>
              </div>
              <div class="meta-tag">
                <span class="tag-lbl">标称目标:</span>
                <span class="tag-val font-mono">{{ verificationReport.claimed_target }}</span>
              </div>
              <div class="meta-tag">
                <span class="tag-lbl">估算量化级别:</span>
                <span class="tag-val font-mono">{{ verificationReport.quantization_estimate }}</span>
              </div>
              <div class="meta-tag">
                <span class="tag-lbl">总探测耗时:</span>
                <span class="tag-val font-mono">{{ verificationReport.total_latency_ms }} ms</span>
              </div>
            </div>

            <div v-if="verificationReport.risk_tags && verificationReport.risk_tags.length > 0" class="risk-tags-row">
              <span class="risk-label">风险警报:</span>
              <span
                v-for="tag in verificationReport.risk_tags"
                :key="tag"
                class="risk-chip"
              >
                ⚠️ {{ tag }}
              </span>
            </div>
          </div>

          <div class="cert-action-col">
            <button class="btn btn-secondary btn-copy-report" @click="copyReportMarkdown">
              {{ isCopied ? '✓ 已复制完整凭证' : '📋 复制验真凭证' }}
            </button>
          </div>
        </div>

        <!-- Probes Detailed Breakdown -->
        <div class="probes-table-card">
          <div class="table-card-header">
            <h4>各项高敏验真探针响应明细</h4>
            <span class="header-hint">点击任意行可查看模型原生回复与判断逻辑</span>
          </div>

          <div class="probes-table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th style="width: 28%;">探针名称与测试意图</th>
                  <th style="width: 16%;">探测维度</th>
                  <th style="width: 12%;">测试状态</th>
                  <th style="width: 10%;">耗时</th>
                  <th style="width: 34%;">指纹匹配研判</th>
                </tr>
              </thead>
              <tbody>
                <template v-for="p in verificationReport.probe_results" :key="p.probe_id">
                  <tr
                    class="probe-row"
                    :class="{ expanded: expandedProbes.has(p.probe_id) }"
                    @click="toggleProbeExpand(p.probe_id)"
                  >
                    <td>
                      <div class="probe-title-cell">
                        <span class="probe-title">{{ p.title }}</span>
                        <span class="probe-id-sub">{{ p.probe_id }}</span>
                      </div>
                    </td>
                    <td>
                      <span class="badge badge-info">{{ getCategoryLabel(p.category) }}</span>
                    </td>
                    <td>
                      <span class="badge" :class="p.passed ? 'badge-success' : 'badge-danger'">
                        {{ p.passed ? '✓ 通过' : '✕ 未通过' }}
                      </span>
                    </td>
                    <td class="font-mono">{{ p.latency_ms }} ms</td>
                    <td>
                      <div class="findings-cell" :class="{ 'text-rose': !p.passed, 'text-emerald': p.passed }">
                        {{ p.findings }}
                      </div>
                    </td>
                  </tr>

                  <!-- Expanded Probe Detail -->
                  <tr v-if="expandedProbes.has(p.probe_id)" class="probe-detail-row">
                    <td colspan="5">
                      <div class="probe-expanded-box">
                        <div class="probe-box-section">
                          <span class="section-lbl">📥 探针输入 Prompt:</span>
                          <div class="code-box prompt-box">{{ p.prompt }}</div>
                        </div>
                        <div class="probe-box-section">
                          <span class="section-lbl">📤 模型原始输出 (Raw Output):</span>
                          <div class="code-box output-box">{{ p.response_text }}</div>
                        </div>
                      </div>
                    </td>
                  </tr>
                </template>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Baseline Drift Card in Fast Probe Mode -->
        <div v-if="verificationReport.baseline_drift && verificationReport.baseline_drift.baseline_found" class="drift-table-card">
          <div class="table-card-header">
            <h4>🏛️ 服务端 verified 基线对照与漂移提示</h4>
            <span class="header-hint">与 {{ verificationReport.baseline_drift.vendor }} 原厂公布基准分自动锚定</span>
          </div>

          <div class="probes-table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>评测项目</th>
                  <th>分类领域</th>
                  <th>官方标准公布分</th>
                  <th>实测参考分</th>
                  <th>偏离度 (Δ)</th>
                  <th>状态</th>
                  <th>解读</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="c in verificationReport.baseline_drift.comparisons" :key="c.benchmark_id">
                  <td class="font-bold text-sky">{{ c.benchmark_name }}</td>
                  <td>{{ c.category }}</td>
                  <td class="font-mono">{{ c.official_score.toFixed(1) }} {{ c.unit }}</td>
                  <td class="font-mono">{{ c.tested_score.toFixed(1) }} {{ c.unit }}</td>
                  <td class="font-mono" :class="c.delta_pct < -15 ? 'text-rose font-bold' : (c.delta_pct < -5 ? 'text-gold' : 'text-emerald')">
                    {{ c.delta_pct > 0 ? '+' : '' }}{{ c.delta_pct.toFixed(1) }}%
                  </td>
                  <td>
                    <span
                      class="badge"
                      :class="{
                        'badge-success': c.status === 'Matching',
                        'badge-warning': c.status === 'MinorDeficit',
                        'badge-danger': c.status === 'SevereDeficit',
                        'badge-info': c.status === 'SuspiciousOverfit'
                      }"
                    >
                      {{ getDriftBadgeLabel(c.status) }}
                    </span>
                  </td>
                  <td class="text-subtle">{{ c.diagnosis }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <!-- ========================================================================= -->
    <!-- TAB 3: 🏛️ Official Ground Truth Baselines Database                         -->
    <!-- ========================================================================= -->
    <div v-if="activeMode === 'baselines'" class="mode-content-block fade-in">
      <div v-if="baselineLoadError" class="error-banner">⚠️ 基线库加载失败：{{ baselineLoadError }}</div>
      <div v-else-if="baselineLoading" class="config-hint">正在加载服务端基线库...</div>
      <div v-else-if="officialBaselines.length === 0" class="config-hint">服务端没有提供基线数据。</div>
      <div class="baselines-overview-card">
        <div class="overview-header">
          <div>
            <h4 class="text-gold">🏛️ 服务端基线库（含来源核实状态）</h4>
            <p class="text-subtle">
              基线来自服务端 <code>/api/baselines</code>，每条带 <strong>verified</strong> 标记。未核实/链接失效的条目不会参与漂移判定，也不应当作“官方绝对锚点”。
            </p>
          </div>
          <div class="baseline-model-nav">
            <button
              v-for="b in officialBaselines"
              :key="b.canonical_id"
              class="canonical-chip"
              :class="{ active: selectedBaselineModel === b.canonical_id }"
              @click="selectedBaselineModel = b.canonical_id"
            >
              {{ b.display_name }}
            </button>
          </div>
        </div>

        <!-- Active Model Baseline Detail Card -->
        <div v-if="activeBaseline" class="active-baseline-detail">
          <div class="baseline-profile-ribbon">
            <div class="profile-main">
              <span class="profile-name">{{ activeBaseline.display_name }}</span>
              <span class="profile-vendor">{{ activeBaseline.vendor }} · 发布于 {{ activeBaseline.release_date }}</span>
              <span v-if="activeBaseline.verified === false" class="badge badge-warning">未核实</span>
              <span v-else-if="activeBaseline.verified" class="badge badge-success">已核实</span>
            </div>
            <div class="profile-report">
              <span class="report-label">官方论文 / 报告:</span>
              <a v-if="activeBaseline.tech_report_url" :href="activeBaseline.tech_report_url" target="_blank" class="report-link">
                {{ activeBaseline.tech_report_title }} ↗
              </a>
              <span v-else class="text-subtle">未提供可核实链接</span>
            </div>
          </div>

          <!-- Traits Chips -->
          <div class="traits-row">
            <span class="traits-label">原生核心指纹特征:</span>
            <span v-for="(trait, idx) in activeBaseline.primary_traits" :key="idx" class="trait-chip">
              🛡️ {{ trait }}
            </span>
          </div>

          <!-- Scores Grid Cards -->
          <div class="baseline-scores-grid">
            <div
              v-for="sc in activeBaseline.scores"
              :key="sc.benchmark_id"
              class="b-score-card"
            >
              <div class="b-card-top">
                <span class="b-cat-badge">{{ sc.category }}</span>
                <span class="b-unit">{{ sc.unit }}</span>
              </div>
              <div class="b-name">{{ sc.benchmark_name }}</div>
              <div class="b-val">{{ sc.score.toFixed(1) }}</div>
              <div class="b-citation" :title="sc.citation">出处: {{ sc.citation }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, computed, onMounted } from 'vue';
import { CLAIMED_TARGET_OPTIONS } from '../utils/benchmark.js';

const claimedTargets = CLAIMED_TARGET_OPTIONS;

const props = defineProps({
  models: {
    type: Array,
    default: () => []
  }
});

const activeMode = ref('agent'); // 'agent' | 'probe' | 'baselines'
const formModel = ref(props.models[0]?.model_id || 'mock-pro');
const formTarget = ref('DeepSeek-R1');
const formBaseUrl = ref('');
const formApiKey = ref('');

// Probe mode states
const isVerifying = ref(false);
const verificationReport = ref(null);
const verifyError = ref('');
const expandedProbes = ref(new Set());
const isCopied = ref(false);

// Agent mode states
const isAgentAuditing = ref(false);
const agentAuditReport = ref(null);
const agentAuditError = ref('');

// Baselines mode states
const officialBaselines = ref([]);
const baselineLoading = ref(true);
const baselineLoadError = ref('');
const selectedBaselineModel = ref('deepseek-r1');

const activeBaseline = computed(() => {
  return officialBaselines.value.find(b => b.canonical_id === selectedBaselineModel.value) || officialBaselines.value[0];
});

watch(() => props.models, (newModels) => {
  if (newModels.length > 0 && !formModel.value) {
    formModel.value = newModels[0].model_id;
  }
});

function toggleProbeExpand(id) {
  if (expandedProbes.value.has(id)) {
    expandedProbes.value.delete(id);
  } else {
    expandedProbes.value.add(id);
  }
}

function getCategoryLabel(cat) {
  const map = {
    identity: '创作者身份',
    capability_cliff: '旗舰能力断崖',
    stylistic: '思考链格式',
    quantization_sensitivity: '浮点量化截断',
    wrapper_leakage: '代理提示词注入'
  };
  return map[cat] || cat;
}

function getDriftBadgeLabel(status) {
  const map = {
    Matching: '✓ 吻合基线 (±5%)',
    MinorDeficit: '⚠️ 轻微偏离 (-5%~-15%)',
    SevereDeficit: '🚨 严重衰减 (<-15%)',
    SuspiciousOverfit: '❓ 异常高分 (+10%)'
  };
  return map[status] || status;
}

// ==========================================
// 1. Agentic Verification Action
// ==========================================
async function startAgentAudit() {
  isAgentAuditing.value = true;
  isCopied.value = false;
  agentAuditError.value = '';

  try {
    const res = await fetch('/api/verify/agentic', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: formModel.value,
        target: formTarget.value,
        base_url: formBaseUrl.value.trim() || null,
        api_key: formApiKey.value.trim() || null,
      })
    });

    if (!res.ok) {
      throw new Error(`HTTP ${res.status} Agent 审计失败`);
    }

    const data = await res.json();
    if (data.error) {
      throw new Error(data.error);
    }

    agentAuditReport.value = data;
    agentAuditError.value = '';
  } catch (err) {
    console.warn('Backend API error', err);
    agentAuditReport.value = null;
    agentAuditError.value = err.message || '验真请求失败';
  } finally {
    isAgentAuditing.value = false;
  }
}

// ==========================================
// 2. Fast Static Probe Verification Action
// ==========================================
async function startVerification() {
  isVerifying.value = true;
  isCopied.value = false;
  verifyError.value = '';

  try {
    const res = await fetch('/api/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: formModel.value,
        target: formTarget.value,
        base_url: formBaseUrl.value.trim() || null,
        api_key: formApiKey.value.trim() || null,
      })
    });

    if (!res.ok) {
      throw new Error(`HTTP ${res.status} 验真请求失败`);
    }

    const data = await res.json();
    if (data.error) {
      throw new Error(data.error);
    }

    verificationReport.value = data;
    verifyError.value = '';
  } catch (err) {
    console.warn('API error', err);
    verificationReport.value = null;
    verifyError.value = err.message || '验真请求失败';
  } finally {
    isVerifying.value = false;
  }
}

function signalStatus(status) {
  if (status === 'passed') return '通过';
  if (status === 'failed') return '失败';
  return '不确定';
}

function copyReportMarkdown() {
  if (!verificationReport.value) return;
  const r = verificationReport.value;
  const md = `# 🛡️ Agent-Bench 多信号证据核验报告（非认证）
- **测试模型标识**: ${r.model_id}
- **标称目标**: ${r.claimed_target}
- **证据分**: ${(r.evidence_score ?? r.authenticity_score).toFixed(1)} / 100.0
- **核验结论**: ${r.verdict_title}
- **估算量化级别**: ${r.quantization_estimate}
- **验真时间**: ${r.verified_at}
- **总耗时**: ${r.total_latency_ms} ms
${r.risk_tags && r.risk_tags.length ? `\n> ⚠️ **潜在风险**: ${r.risk_tags.join(' | ')}` : ''}

### 【综合分析摘要】
${r.summary}

### 【高敏探针检测明细】
| 探针名称 | 探测维度 | 状态 | 耗时 | 指纹分析结论 |
|---|---|---|---|---|
${r.probe_results.map(p => `| ${p.title} | ${getCategoryLabel(p.category)} | ${p.passed ? '✓ 通过' : '✕ 未通过'} | ${p.latency_ms}ms | ${p.findings} |`).join('\n')}
`;

  navigator.clipboard.writeText(md).then(() => {
    isCopied.value = true;
    setTimeout(() => { isCopied.value = false; }, 2500);
  });
}

function copyAgentReportMarkdown() {
  if (!agentAuditReport.value) return;
  const r = agentAuditReport.value;
  const md = `# 🤖 Agent-Bench 动态证据审计报告（非认证）
- **审计单号**: ${r.audit_id}
- **目标端点模型**: ${r.target_model}
- **参考基线**: ${r.canonical_baseline_name}
- **证据分**: ${(r.evidence_score ?? r.authenticity_score).toFixed(1)} / 100.0
- **证据裁定**: ${r.verdict_title}
- **阿谀附和指数**: ${r.sycophancy_index.toFixed(1)}% (${r.sycophancy_assessment})
- **总耗时**: ${r.total_latency_ms} ms

### 【Agent 裁决摘要】
${r.executive_summary}

### 【现场动态对抗题目】
| 题目名称 | 陷阱机制 | 状态 | CoT思考链 | 审计点评 |
|---|---|---|---|---|
${r.dynamic_probes.map(p => `| ${p.title} | ${p.trap_type} | ${p.passed ? '✓ 通过' : '✕ 踩雷'} | ${p.has_cot_signature ? '具备' : '-'} | ${p.auditor_critique} |`).join('\n')}

### 【多轮交叉审讯式反问记录】
${r.cross_examinations.map((ce, i) => `
#### 轮次 ${i + 1}: ${ce.probe_title}
- **审讯员诱导**: ${ce.interrogator_challenge}
- **模型辩护回复**: ${ce.target_defense_response}
- **审计点评**: ${ce.auditor_notes}
`).join('\n')}
`;

  navigator.clipboard.writeText(md).then(() => {
    isCopied.value = true;
    setTimeout(() => { isCopied.value = false; }, 2500);
  });
}

onMounted(async () => {
  try {
    const res = await fetch('/api/baselines');
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    officialBaselines.value = Array.isArray(data) ? data : [];
    if (officialBaselines.value.length > 0
        && !officialBaselines.value.some(b => b.canonical_id === selectedBaselineModel.value)) {
      selectedBaselineModel.value = officialBaselines.value[0].canonical_id;
    }
  } catch (err) {
    baselineLoadError.value = err.message || '无法加载服务端基线库';
  } finally {
    baselineLoading.value = false;
  }
});
</script>

<style scoped>
.model-verification-container {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

/* Header */
.verify-header-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  padding: 1.25rem 1.75rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}

.header-left {
  max-width: 720px;
}

.panel-title {
  margin: 0 0 0.4rem 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-main);
}

.panel-subtitle {
  margin: 0;
  font-size: 0.88rem;
  color: var(--text-subtle);
  line-height: 1.5;
}

/* Tab Switcher */
.tab-switcher {
  display: flex;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 4px;
  gap: 4px;
}

.tab-btn {
  background: transparent;
  border: none;
  color: var(--text-subtle);
  padding: 8px 16px;
  border-radius: 7px;
  font-size: 0.88rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tab-btn:hover {
  color: var(--text-main);
  background: rgba(255, 255, 255, 0.05);
}

.tab-btn.active {
  background: linear-gradient(135deg, rgba(56, 189, 248, 0.2) 0%, rgba(99, 102, 241, 0.25) 100%);
  color: #38bdf8;
  border: 1px solid rgba(56, 189, 248, 0.4);
  box-shadow: 0 2px 8px rgba(56, 189, 248, 0.2);
}

/* Mode Block */
.mode-content-block {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

/* Config Card */
.config-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  padding: 1.25rem 1.5rem;
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 1rem;
}

.config-item {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.config-item label {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-subtle);
}

.cfg-select,
.cfg-input {
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid var(--border-soft);
  color: var(--text-main);
  border-radius: 8px;
  padding: 0.6rem 0.8rem;
  font-size: 0.88rem;
  transition: border-color 0.2s;
}

.cfg-select:focus,
.cfg-input:focus {
  outline: none;
  border-color: #38bdf8;
}

.config-actions {
  margin-top: 1rem;
  display: flex;
  justify-content: flex-end;
}

.btn-start-agent,
.btn-start-verify {
  padding: 0.75rem 1.75rem;
  font-size: 0.95rem;
  font-weight: 700;
  border-radius: 9px;
  background: linear-gradient(135deg, #0284c7 0%, #4f46e5 100%);
  color: #fff;
  border: none;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(79, 70, 229, 0.35);
  transition: transform 0.15s, opacity 0.2s;
}

.btn-start-agent:hover,
.btn-start-verify:hover {
  transform: translateY(-1px);
}

.btn-start-agent:disabled,
.btn-start-verify:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

/* Scanning Card */
.scanning-card {
  background: var(--bg-card);
  border: 1px solid rgba(56, 189, 248, 0.3);
  border-radius: 14px;
  padding: 2.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 2.5rem;
  box-shadow: 0 0 25px rgba(56, 189, 248, 0.1);
}

.radar-scan-box {
  position: relative;
  width: 110px;
  height: 110px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(56, 189, 248, 0.08) 0%, rgba(15, 23, 42, 0.8) 70%);
  border: 1px solid rgba(56, 189, 248, 0.4);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.radar-ring {
  position: absolute;
  border: 1px dashed rgba(56, 189, 248, 0.3);
  border-radius: 50%;
}

.radar-ring.r1 { width: 35px; height: 35px; }
.radar-ring.r2 { width: 70px; height: 70px; }
.radar-ring.r3 { width: 100px; height: 100px; }

.radar-sweep {
  position: absolute;
  width: 50%;
  height: 50%;
  top: 0;
  left: 0;
  transform-origin: bottom right;
  background: linear-gradient(45deg, rgba(56, 189, 248, 0.6), transparent);
  animation: radar-spin 1.8s linear infinite;
}

.agent-sweep {
  background: linear-gradient(45deg, rgba(236, 72, 153, 0.6), transparent);
}

.radar-center-icon {
  font-size: 1.8rem;
  z-index: 2;
}

@keyframes radar-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.scanning-info h4 {
  margin: 0 0 0.5rem 0;
  font-size: 1.1rem;
  color: #38bdf8;
}

.scanning-info p {
  margin: 0;
  font-size: 0.85rem;
  color: var(--text-subtle);
  line-height: 1.6;
}

/* Certificate Card */
.certificate-card {
  border-radius: 14px;
  border: 1px solid var(--border-soft);
  background: var(--bg-card);
  padding: 1.5rem 1.75rem;
  display: flex;
  gap: 1.75rem;
  position: relative;
  overflow: hidden;
}

.cert-consistent {
  border-color: rgba(46, 204, 113, 0.55) !important;
  background: linear-gradient(135deg, rgba(46, 204, 113, 0.12), rgba(20, 25, 35, 0.5)) !important;
}

.cert-inconclusive {
  border-color: rgba(241, 196, 15, 0.55) !important;
  background: linear-gradient(135deg, rgba(241, 196, 15, 0.12), rgba(20, 25, 35, 0.5)) !important;
}

.cert-suspicious {
  border-color: rgba(230, 126, 34, 0.55) !important;
  background: linear-gradient(135deg, rgba(230, 126, 34, 0.12), rgba(20, 25, 35, 0.5)) !important;
}

.cert-highrisk {
  border-color: rgba(231, 76, 60, 0.6) !important;
  background: linear-gradient(135deg, rgba(231, 76, 60, 0.14), rgba(20, 25, 35, 0.5)) !important;
}

.signal-matrix {
  margin-top: 12px;
  border: 1px solid var(--border-color, rgba(255,255,255,0.12));
  border-radius: 10px;
  padding: 10px 12px;
  background: rgba(0, 0, 0, 0.12);
}

.signal-matrix h5 {
  margin: 0 0 8px;
  font-size: 12px;
  color: var(--text-secondary, #aab);
  letter-spacing: .04em;
}

.signal-row {
  display: grid;
  grid-template-columns: 190px 60px 1fr;
  gap: 8px;
  align-items: start;
  padding: 4px 0;
  border-top: 1px dashed rgba(255,255,255,0.07);
  font-size: 12px;
}

.signal-name { color: var(--text-primary, #e8e8e8); }
.signal-status { font-weight: 600; }
.signal-detail { color: var(--text-secondary, #98a0aa); }
.signal-passed .signal-status { color: #2ecc71; }
.signal-failed .signal-status { color: #e74c3c; }
.signal-inconclusive .signal-status { color: #f1c40f; }

.cert-authentic,
.cert-authenticverified {
  border-color: rgba(16, 185, 129, 0.5);
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.08) 0%, rgba(15, 23, 42, 0.95) 60%);
  box-shadow: 0 4px 20px rgba(16, 185, 129, 0.15);
}

.cert-suspiciousquantizationorvariant,
.cert-probablequantizeddrop {
  border-color: rgba(245, 158, 11, 0.5);
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.08) 0%, rgba(15, 23, 42, 0.95) 60%);
  box-shadow: 0 4px 20px rgba(245, 158, 11, 0.15);
}

.cert-highriskspoofing,
.cert-highriskcounterfeit {
  border-color: rgba(239, 68, 68, 0.5);
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.1) 0%, rgba(15, 23, 42, 0.95) 60%);
  box-shadow: 0 4px 20px rgba(239, 68, 68, 0.2);
}

.score-dial {
  width: 95px;
  height: 95px;
  border-radius: 50%;
  border: 3px solid rgba(56, 189, 248, 0.6);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.8);
}

.dial-num {
  font-size: 1.75rem;
  font-weight: 800;
  color: #38bdf8;
  line-height: 1;
}

.dial-unit {
  font-size: 0.68rem;
  color: var(--text-subtle);
  margin-top: 4px;
}

.cert-main-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.cert-title-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}

.cert-verdict-title {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-main);
}

.cert-time {
  font-size: 0.8rem;
  color: var(--text-subtle);
}

.cert-summary {
  margin: 0;
  font-size: 0.92rem;
  color: var(--text-main);
  line-height: 1.5;
}

.cert-meta-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  margin-top: 0.25rem;
}

.meta-tag {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  background: rgba(255, 255, 255, 0.04);
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 0.8rem;
}

.tag-lbl {
  color: var(--text-subtle);
}

.tag-val {
  color: var(--text-main);
  font-weight: 600;
}

.risk-tags-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-top: 0.4rem;
}

.risk-label {
  font-size: 0.8rem;
  font-weight: 700;
  color: #f87171;
}

.risk-chip {
  background: rgba(239, 68, 68, 0.15);
  border: 1px solid rgba(239, 68, 68, 0.35);
  color: #f87171;
  font-size: 0.78rem;
  padding: 3px 8px;
  border-radius: 6px;
}

.cert-action-col {
  display: flex;
  align-items: flex-start;
}

.btn-copy-report {
  white-space: nowrap;
  padding: 0.6rem 1.1rem;
  font-size: 0.85rem;
  border-radius: 8px;
}

/* Probes & Drift Tables */
.probes-table-card,
.drift-table-card,
.interrogation-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  padding: 1.25rem 1.5rem;
}

.table-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.table-card-header h4 {
  margin: 0;
  font-size: 1.05rem;
  color: var(--text-main);
}

.header-hint {
  font-size: 0.8rem;
  color: var(--text-subtle);
}

.probes-table-wrap {
  overflow-x: auto;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.88rem;
}

.data-table th {
  background: rgba(15, 23, 42, 0.75);
  color: var(--text-subtle);
  font-weight: 600;
  text-align: left;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--border-soft);
}

.data-table td {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  vertical-align: middle;
}

.probe-row {
  cursor: pointer;
  transition: background 0.15s;
}

.probe-row:hover {
  background: rgba(255, 255, 255, 0.03);
}

.probe-title-cell {
  display: flex;
  flex-direction: column;
}

.probe-title {
  font-weight: 600;
  color: var(--text-main);
}

.probe-id-sub {
  font-size: 0.75rem;
  color: var(--text-subtle);
  font-family: monospace;
}

.probe-expanded-box {
  background: rgba(15, 23, 42, 0.8);
  border-radius: 8px;
  padding: 1rem;
  margin: 0.5rem 0;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.section-lbl {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--text-subtle);
  margin-bottom: 0.25rem;
  display: block;
}

.code-box {
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 0.6rem 0.8rem;
  font-family: monospace;
  font-size: 0.82rem;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 180px;
  overflow-y: auto;
}

/* Interrogation Card */
.dialogue-list {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.dialogue-item {
  background: rgba(15, 23, 42, 0.5);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.dialogue-warn {
  border-color: rgba(239, 68, 68, 0.4);
  background: rgba(239, 68, 68, 0.04);
}

.dialogue-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.dialogue-step-badge {
  font-size: 0.75rem;
  font-weight: 700;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.15);
  padding: 2px 8px;
  border-radius: 4px;
}

.dialogue-probe-title {
  font-weight: 700;
  font-size: 0.95rem;
  color: var(--text-main);
  flex: 1;
}

.chat-thread {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.bubble-row {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.bubble-avatar {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--text-subtle);
}

.bubble-content {
  border-radius: 10px;
  padding: 0.75rem 1rem;
  font-size: 0.88rem;
  line-height: 1.5;
}

.bubble-auditor-content {
  background: rgba(239, 68, 68, 0.08);
  border: 1px solid rgba(239, 68, 68, 0.25);
  color: #fca5a5;
}

.bubble-target-content {
  background: rgba(56, 189, 248, 0.08);
  border: 1px solid rgba(56, 189, 248, 0.25);
  color: var(--text-main);
}

.dialogue-verdict-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  font-size: 0.85rem;
  font-weight: 600;
}

.bar-success {
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
}

.bar-danger {
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
}

/* Drift Summary Ribbon */
.drift-summary-ribbon {
  display: flex;
  flex-wrap: wrap;
  gap: 1.5rem;
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  padding: 0.75rem 1.25rem;
  margin-bottom: 1rem;
}

.ribbon-item {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.85rem;
}

.r-lbl {
  color: var(--text-subtle);
}

.r-val {
  color: var(--text-main);
}

/* Baselines View */
.baselines-overview-card {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.overview-header {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.baseline-model-nav {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.canonical-chip {
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid var(--border-soft);
  color: var(--text-subtle);
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.canonical-chip:hover {
  color: var(--text-main);
  background: rgba(255, 255, 255, 0.06);
}

.canonical-chip.active {
  background: linear-gradient(135deg, rgba(234, 179, 8, 0.2) 0%, rgba(249, 115, 22, 0.25) 100%);
  color: #fbbf24;
  border-color: rgba(234, 179, 8, 0.5);
  box-shadow: 0 2px 8px rgba(234, 179, 8, 0.2);
}

.active-baseline-detail {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.baseline-profile-ribbon {
  background: rgba(15, 23, 42, 0.8);
  border: 1px solid rgba(234, 179, 8, 0.3);
  border-radius: 10px;
  padding: 1rem 1.25rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.profile-main {
  display: flex;
  flex-direction: column;
}

.profile-name {
  font-size: 1.3rem;
  font-weight: 800;
  color: #fbbf24;
}

.profile-vendor {
  font-size: 0.82rem;
  color: var(--text-subtle);
}

.profile-report {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.85rem;
}

.report-label {
  color: var(--text-subtle);
}

.report-link {
  color: #38bdf8;
  font-weight: 600;
  text-decoration: none;
}

.report-link:hover {
  text-decoration: underline;
}

.traits-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.traits-label {
  font-size: 0.85rem;
  font-weight: 700;
  color: var(--text-subtle);
}

.trait-chip {
  background: rgba(56, 189, 248, 0.1);
  border: 1px solid rgba(56, 189, 248, 0.3);
  color: #7dd3fc;
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 0.8rem;
}

.baseline-scores-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 1rem;
}

.b-score-card {
  background: rgba(15, 23, 42, 0.7);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 1rem 1.1rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  transition: transform 0.15s, border-color 0.2s;
}

.b-score-card:hover {
  transform: translateY(-2px);
  border-color: rgba(56, 189, 248, 0.4);
}

.b-card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.b-cat-badge {
  font-size: 0.7rem;
  color: #94a3b8;
  background: rgba(255, 255, 255, 0.05);
  padding: 2px 6px;
  border-radius: 4px;
}

.b-unit {
  font-size: 0.75rem;
  font-family: monospace;
  color: var(--text-subtle);
}

.b-name {
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--text-main);
}

.b-val {
  font-size: 1.6rem;
  font-weight: 800;
  color: #38bdf8;
  font-family: monospace;
}

.b-citation {
  font-size: 0.72rem;
  color: var(--text-subtle);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Helpers */
.text-rose { color: #f87171 !important; }
.text-emerald { color: #34d399 !important; }
.text-sky { color: #38bdf8 !important; }
.text-gold { color: #fbbf24 !important; }
.text-subtle { color: var(--text-subtle) !important; }
.font-mono { font-family: monospace; }
.font-bold { font-weight: 700; }
.badge {
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
}
.badge-success { background: rgba(16, 185, 129, 0.15); color: #34d399; }
.badge-danger { background: rgba(239, 68, 68, 0.15); color: #f87171; }
.badge-warning { background: rgba(245, 158, 11, 0.15); color: #fbbf24; }
.badge-info { background: rgba(56, 189, 248, 0.15); color: #38bdf8; }
.badge-neutral { background: rgba(255, 255, 255, 0.06); color: var(--text-subtle); }

.spinner-sm {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-radius: 50%;
  border-top-color: #fff;
  animation: spin 0.8s linear infinite;
  vertical-align: middle;
  margin-right: 6px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.fade-in {
  animation: fadeIn 0.25s ease-out forwards;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}
.error-banner {
  margin: 1rem 0;
  padding: 0.9rem 1.1rem;
  border-radius: 12px;
  border: 1px solid rgba(244, 63, 94, 0.45);
  background: rgba(244, 63, 94, 0.10);
  color: #fda4af;
  font-size: 0.9rem;
}
.config-hint {
  margin-top: 0.6rem;
  color: var(--text-faint, #94a3b8);
  font-size: 0.82rem;
  line-height: 1.5;
}
</style>
