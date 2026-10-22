use crate::control::RunControl;
use eval_agent::{
    get_pi_tools, AgentRunner, SimulatedEnvironment, TrajectoryEvaluator, WorkspaceEnv,
};
use eval_core::dataset::{Category, Dataset, EvaluationType, TestCase};
use eval_core::evaluators::{
    CodeSandboxEvaluator, EvaluationResult, Evaluator, ExactMatchEvaluator, F1Evaluator,
    IfEvalEvaluator, JsonSchemaEvaluator, LlmJudgeEvaluator, MultipleChoiceEvaluator,
    NumericEvaluator, RegexEvaluator, ToolCallEvaluator,
};
use eval_core::metrics::{CaseResult, ModelBenchmarkSummary};
use eval_core::model::{ChatMessage, ModelClient};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct BenchmarkOrchestrator {
    concurrency: usize,
    judge_client: Option<Arc<dyn ModelClient>>,
    control: Arc<RunControl>,
}

#[derive(Clone)]
struct CaseMeta {
    id: String,
    name: Option<String>,
    category: Category,
    difficulty: Option<String>,
}

impl From<&TestCase> for CaseMeta {
    fn from(tc: &TestCase) -> Self {
        Self {
            id: tc.id.clone(),
            name: tc.name.clone(),
            category: tc.category.clone(),
            difficulty: tc.difficulty.clone(),
        }
    }
}

impl BenchmarkOrchestrator {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency: concurrency.max(1),
            judge_client: None,
            control: Arc::new(RunControl::new()),
        }
    }

    pub fn with_judge(mut self, judge: Arc<dyn ModelClient>) -> Self {
        self.judge_client = Some(judge);
        self
    }

    pub fn with_control(mut self, control: Arc<RunControl>) -> Self {
        self.control = control;
        self
    }

    pub fn control(&self) -> Arc<RunControl> {
        Arc::clone(&self.control)
    }

    pub async fn run_benchmark(
        &self,
        client: Arc<dyn ModelClient>,
        dataset: &Dataset,
    ) -> anyhow::Result<ModelBenchmarkSummary> {
        let mp = MultiProgress::new();
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        self.run_benchmark_inner(client, dataset, &mp, semaphore)
            .await
    }

    pub async fn run_benchmark_with_progress(
        &self,
        client: Arc<dyn ModelClient>,
        dataset: &Dataset,
        mp: &MultiProgress,
    ) -> anyhow::Result<ModelBenchmarkSummary> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        self.run_benchmark_inner(client, dataset, mp, semaphore)
            .await
    }

    /// Run all models through one shared semaphore so `concurrency` is a true
    /// global cap. A model failure is returned as an error for that model
    /// instead of being silently dropped.
    pub async fn run_all_models_parallel(
        &self,
        clients: Vec<Arc<dyn ModelClient>>,
        dataset: &Dataset,
    ) -> anyhow::Result<Vec<(String, anyhow::Result<ModelBenchmarkSummary>)>> {
        let mp = Arc::new(MultiProgress::new());
        let arc_self = Arc::new(self.clone());
        let dataset_arc = Arc::new(dataset.clone());
        let semaphore = Arc::new(Semaphore::new(self.concurrency));

        let mut handles: Vec<(String, JoinHandle<anyhow::Result<ModelBenchmarkSummary>>)> =
            Vec::new();
        for client in clients {
            let orch = Arc::clone(&arc_self);
            let ds = Arc::clone(&dataset_arc);
            let mp_clone = Arc::clone(&mp);
            let sem = Arc::clone(&semaphore);
            let model_id = client.config().id.clone();
            let handle =
                tokio::spawn(
                    async move { orch.run_benchmark_inner(client, &ds, &mp_clone, sem).await },
                );
            handles.push((model_id, handle));
        }

        let mut outcomes: Vec<(String, anyhow::Result<ModelBenchmarkSummary>)> = Vec::new();
        for (model_id, handle) in handles {
            match handle.await {
                Ok(res) => outcomes.push((model_id, res)),
                Err(join_err) => outcomes.push((
                    model_id,
                    Err(anyhow::anyhow!("model evaluation task failed: {join_err}")),
                )),
            }
        }

        // Elo needs all successful summaries at once.
        let ok_indices: Vec<usize> = outcomes
            .iter()
            .enumerate()
            .filter(|(_, (_, res))| res.is_ok())
            .map(|(idx, _)| idx)
            .collect();
        let mut summaries: Vec<ModelBenchmarkSummary> = ok_indices
            .iter()
            .map(|&idx| outcomes[idx].1.as_ref().unwrap().clone())
            .collect();
        ModelBenchmarkSummary::compute_head_to_head_elo(&mut summaries);
        for (idx, summary) in ok_indices.into_iter().zip(summaries) {
            outcomes[idx].1 = Ok(summary);
        }

        Ok(outcomes)
    }

    async fn run_benchmark_inner(
        &self,
        client: Arc<dyn ModelClient>,
        dataset: &Dataset,
        mp: &MultiProgress,
        semaphore: Arc<Semaphore>,
    ) -> anyhow::Result<ModelBenchmarkSummary> {
        let total_cases = dataset.test_cases.len();
        let pb = mp.add(ProgressBar::new(total_cases as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!("Evaluating '{}'", client.config().model_name));

        let mut tasks: Vec<(CaseMeta, JoinHandle<CaseResult>)> = Vec::new();

        for test_case in dataset.test_cases.clone() {
            let meta = CaseMeta::from(&test_case);
            let task_meta = meta.clone();
            let client_clone = Arc::clone(&client);
            let judge_clone = self.judge_client.clone();
            let sem_clone = Arc::clone(&semaphore);
            let pb_clone = pb.clone();
            let control = Arc::clone(&self.control);
            let timeout_secs = test_case.timeout_seconds();

            let task = tokio::spawn(async move {
                if control.is_cancelled() {
                    pb_clone.inc(1);
                    return cancelled_result(&task_meta);
                }

                let _permit = tokio::select! {
                    _ = control.cancelled() => {
                        pb_clone.inc(1);
                        return cancelled_result(&task_meta);
                    }
                    permit = sem_clone.acquire() => match permit {
                        Ok(p) => p,
                        Err(_) => {
                            pb_clone.inc(1);
                            return cancelled_result(&task_meta);
                        }
                    }
                };

                let res = tokio::select! {
                    _ = control.cancelled() => cancelled_result(&task_meta),
                    outcome = tokio::time::timeout(
                        Duration::from_secs(timeout_secs),
                        Self::execute_single_case(client_clone, judge_clone, test_case),
                    ) => match outcome {
                        Ok(Ok(case_res)) => case_res,
                        Ok(Err(err)) => error_result(&task_meta, format!("Evaluation error: {err}"), 0),
                        Err(_) => timeout_result(&task_meta, timeout_secs),
                    }
                };
                pb_clone.inc(1);
                res
            });

            tasks.push((meta, task));
        }

        let mut case_results = Vec::new();
        for (meta, task) in tasks {
            match task.await {
                Ok(res) => case_results.push(res),
                Err(join_err) => {
                    tracing::error!("Worker task panicked or aborted: {join_err}");
                    case_results.push(panic_result(&meta, join_err));
                }
            }
        }

        pb.finish_with_message(format!(
            "Completed benchmark for '{}'",
            client.config().model_name
        ));

        Ok(ModelBenchmarkSummary::compute(
            client.config().id.clone(),
            client.config().model_name.clone(),
            case_results,
        ))
    }

    async fn execute_single_case(
        client: Arc<dyn ModelClient>,
        judge_client: Option<Arc<dyn ModelClient>>,
        test_case: TestCase,
    ) -> anyhow::Result<CaseResult> {
        let start_time = Instant::now();

        if test_case.eval_type == EvaluationType::AgentTrajectory {
            let workspace = Arc::new(WorkspaceEnv::new()?);

            if let Some(setup_val) = test_case.metadata.get("setup_files") {
                if let Ok(setup_map) =
                    serde_json::from_value::<HashMap<String, String>>(setup_val.clone())
                {
                    workspace.setup_from_files(&setup_map)?;
                }
            }

            if let Some(tools_val) = test_case
                .metadata
                .get("tool_outputs")
                .or_else(|| test_case.metadata.get("mock_tools"))
            {
                if let Ok(mock_map) =
                    serde_json::from_value::<HashMap<String, String>>(tools_val.clone())
                {
                    workspace.set_mock_tools(mock_map);
                }
            }

            let tools = test_case.tools.clone().unwrap_or_else(get_pi_tools);
            let max_turns = test_case.max_turns.unwrap_or(8).clamp(1, 64);

            let default_sys = eval_core::prompts::AGENT_DEFAULT_SYSTEM.template;
            let sys_prompt = test_case.system_prompt.as_deref().unwrap_or(default_sys);

            let env_client: Arc<dyn SimulatedEnvironment> = workspace.clone();
            let trajectory = AgentRunner::run(
                client.as_ref(),
                Some(sys_prompt),
                &test_case.prompt,
                &tools,
                env_client,
                max_turns,
            )
            .await?;

            let mut eval_res = TrajectoryEvaluator::evaluate_with_judge(
                &test_case,
                &trajectory,
                judge_client.as_ref(),
            )
            .await;

            let verify_cmd = test_case.test_code.as_deref().or_else(|| {
                test_case
                    .metadata
                    .get("verify_cmd")
                    .and_then(|v| v.as_str())
            });

            if let Some(cmd) = verify_cmd {
                let ws = Arc::clone(&workspace);
                let cmd_owned = cmd.to_string();
                let verification =
                    tokio::task::spawn_blocking(move || ws.run_verification(&cmd_owned)).await;
                match verification {
                    Ok(Ok((passed, verify_output))) => {
                        if !passed {
                            eval_res.passed = false;
                            eval_res.score = (eval_res.score * 0.4).min(0.4);
                            eval_res.reason = format!("Verification failed:\n{verify_output}");
                        } else {
                            eval_res.passed = true;
                            eval_res.score = 1.0;
                            eval_res.reason = format!("Verification passed:\n{verify_output}");
                        }
                    }
                    Ok(Err(e)) => {
                        eval_res.passed = false;
                        eval_res.score = 0.0;
                        eval_res.reason = format!("Verification command error: {e}");
                    }
                    Err(join_err) => {
                        eval_res.passed = false;
                        eval_res.score = 0.0;
                        eval_res.reason = format!("Verification worker panicked: {join_err}");
                    }
                }
            }

            let latency_ms = start_time.elapsed().as_millis() as u64;
            let raw_output = trajectory
                .final_answer
                .clone()
                .unwrap_or_else(|| "[No final text returned]".to_string());
            let (extracted_reasoning, clean_output) =
                eval_core::model::extractor::ToolCallExtractor::extract_reasoning_and_clean_text(
                    &raw_output,
                );

            let steps_json = serde_json::to_value(&trajectory.steps)
                .ok()
                .and_then(|v| v.as_array().cloned());

            return Ok(CaseResult {
                test_case_id: test_case.id,
                test_case_name: test_case.name,
                category: test_case.category,
                difficulty: test_case.difficulty,
                passed: eval_res.passed,
                score: eval_res.score,
                dimensions: eval_res.dimensions,
                reason: eval_res.reason,
                latency_ms,
                ttft_ms: None,
                tps: 0.0,
                prompt_tokens: trajectory.prompt_tokens + eval_res.prompt_tokens,
                completion_tokens: trajectory.completion_tokens + eval_res.completion_tokens,
                cost_usd: trajectory.estimated_cost_usd + eval_res.cost_usd,
                model_output: clean_output,
                reasoning_content: extracted_reasoning,
                tool_calls: None,
                finish_reason: None,
                trajectory_steps: steps_json,
                error: None,
            });
        }

        let mut messages = Vec::new();
        if let Some(ref sys) = test_case.system_prompt {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(&test_case.prompt));

        let mut attempts = 0;
        let max_attempts = 3;
        let mut model_resp_opt = None;
        let mut final_error = None;
        let mut usage_prompt_tokens = 0u32;
        let mut usage_completion_tokens = 0u32;
        let mut usage_cost = 0.0f64;

        while attempts < max_attempts {
            attempts += 1;
            match client
                .chat_complete(&messages, test_case.tools.as_deref())
                .await
            {
                Ok(resp) => {
                    usage_prompt_tokens += resp.usage.prompt_tokens;
                    usage_completion_tokens += resp.usage.completion_tokens;
                    usage_cost += resp.estimated_cost_usd;
                    model_resp_opt = Some(resp);
                    break;
                }
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    let is_transient = err_str.contains("429")
                        || err_str.contains("rate limit")
                        || err_str.contains("too many requests")
                        || err_str.contains("503")
                        || err_str.contains("timeout")
                        || err_str.contains("timed out")
                        || err_str.contains("temporarily unavailable");

                    if is_transient && attempts < max_attempts {
                        let backoff_ms = (600 * (1 << attempts)) + (rand::random::<u64>() % 300);
                        tracing::warn!(
                            "Transient/Rate-limit error on case '{}': {}. Backing off {}ms (attempt {}/{})",
                            test_case.id, e, backoff_ms, attempts, max_attempts
                        );
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        continue;
                    }
                    final_error = Some(e);
                    break;
                }
            }
        }

        let model_resp = match model_resp_opt {
            Some(resp) => resp,
            None => {
                let err_msg = final_error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Unknown invocation error".to_string());
                return Ok(CaseResult {
                    test_case_id: test_case.id,
                    test_case_name: test_case.name,
                    category: test_case.category,
                    difficulty: test_case.difficulty,
                    passed: false,
                    score: 0.0,
                    dimensions: None,
                    reason: format!("Model invocation error: {err_msg}"),
                    latency_ms: start_time.elapsed().as_millis() as u64,
                    ttft_ms: None,
                    tps: 0.0,
                    prompt_tokens: usage_prompt_tokens,
                    completion_tokens: usage_completion_tokens,
                    cost_usd: usage_cost,
                    model_output: String::new(),
                    reasoning_content: None,
                    tool_calls: None,
                    finish_reason: None,
                    trajectory_steps: None,
                    error: Some(err_msg),
                });
            }
        };

        let latency_ms = model_resp.total_duration.as_millis() as u64;
        let ttft_ms = model_resp.ttft.map(|t| t.as_millis() as u64);

        // Hard-rule evaluators are authoritative. LLM judge is only used for
        // subjective evaluation types (LlmJudge / AgentTrajectory), never to
        // override a failed exact/regex/schema/code test.
        let eval_res: EvaluationResult = match test_case.eval_type {
            EvaluationType::ExactMatch => {
                ExactMatchEvaluator::default()
                    .evaluate(&test_case, &model_resp)
                    .await?
            }
            EvaluationType::Regex => RegexEvaluator.evaluate(&test_case, &model_resp).await?,
            EvaluationType::Numeric => NumericEvaluator.evaluate(&test_case, &model_resp).await?,
            EvaluationType::MultipleChoice => {
                MultipleChoiceEvaluator
                    .evaluate(&test_case, &model_resp)
                    .await?
            }
            EvaluationType::ToolCall => {
                ToolCallEvaluator
                    .evaluate(&test_case, &model_resp)
                    .await?
            }
            EvaluationType::F1 => F1Evaluator.evaluate(&test_case, &model_resp).await?,
            EvaluationType::IfEval => IfEvalEvaluator.evaluate(&test_case, &model_resp).await?,
            EvaluationType::JsonSchema => {
                JsonSchemaEvaluator
                    .evaluate(&test_case, &model_resp)
                    .await?
            }
            EvaluationType::CodeExecution => {
                CodeSandboxEvaluator::default()
                    .evaluate(&test_case, &model_resp)
                    .await?
            }
            EvaluationType::LlmJudge => match judge_client.as_ref() {
                Some(judge) => {
                    LlmJudgeEvaluator::new(judge.clone())
                        .evaluate(&test_case, &model_resp)
                        .await?
                }
                None => EvaluationResult::fail(
                    "No judge model configured for llm_judge case; refusing to fall back to exact match",
                ),
            },
            EvaluationType::AgentTrajectory => unreachable!(),
        };

        Ok(CaseResult {
            test_case_id: test_case.id,
            test_case_name: test_case.name,
            category: test_case.category,
            difficulty: test_case.difficulty,
            passed: eval_res.passed,
            score: eval_res.score,
            dimensions: eval_res.dimensions,
            reason: eval_res.reason,
            latency_ms,
            ttft_ms,
            tps: model_resp.tokens_per_second,
            prompt_tokens: usage_prompt_tokens + eval_res.prompt_tokens,
            completion_tokens: usage_completion_tokens + eval_res.completion_tokens,
            cost_usd: usage_cost + eval_res.cost_usd,
            model_output: model_resp.text,
            reasoning_content: model_resp.reasoning_content,
            tool_calls: model_resp.tool_calls,
            finish_reason: model_resp.finish_reason,
            trajectory_steps: None,
            error: None,
        })
    }
}

fn cancelled_result(meta: &CaseMeta) -> CaseResult {
    CaseResult {
        test_case_id: meta.id.clone(),
        test_case_name: meta.name.clone(),
        category: meta.category.clone(),
        difficulty: meta.difficulty.clone(),
        passed: false,
        score: 0.0,
        dimensions: None,
        reason: "Run cancelled".to_string(),
        latency_ms: 0,
        ttft_ms: None,
        tps: 0.0,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost_usd: 0.0,
        model_output: String::new(),
        reasoning_content: None,
        tool_calls: None,
        finish_reason: None,
        trajectory_steps: None,
        error: Some("Run cancelled".to_string()),
    }
}

fn error_result(meta: &CaseMeta, reason: String, latency_ms: u64) -> CaseResult {
    CaseResult {
        test_case_id: meta.id.clone(),
        test_case_name: meta.name.clone(),
        category: meta.category.clone(),
        difficulty: meta.difficulty.clone(),
        passed: false,
        score: 0.0,
        dimensions: None,
        reason: reason.clone(),
        latency_ms,
        ttft_ms: None,
        tps: 0.0,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost_usd: 0.0,
        model_output: String::new(),
        reasoning_content: None,
        tool_calls: None,
        finish_reason: None,
        trajectory_steps: None,
        error: Some(reason),
    }
}

fn timeout_result(meta: &CaseMeta, timeout_secs: u64) -> CaseResult {
    let reason = format!("Evaluation timed out after {timeout_secs}s");
    CaseResult {
        test_case_id: meta.id.clone(),
        test_case_name: meta.name.clone(),
        category: meta.category.clone(),
        difficulty: meta.difficulty.clone(),
        passed: false,
        score: 0.0,
        dimensions: None,
        reason: reason.clone(),
        latency_ms: timeout_secs * 1000,
        ttft_ms: None,
        tps: 0.0,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost_usd: 0.0,
        model_output: String::new(),
        reasoning_content: None,
        tool_calls: None,
        finish_reason: None,
        trajectory_steps: None,
        error: Some(reason),
    }
}

fn panic_result(meta: &CaseMeta, join_err: tokio::task::JoinError) -> CaseResult {
    let reason = format!("Worker task panicked or was aborted: {join_err}");
    CaseResult {
        test_case_id: meta.id.clone(),
        test_case_name: meta.name.clone(),
        category: meta.category.clone(),
        difficulty: meta.difficulty.clone(),
        passed: false,
        score: 0.0,
        dimensions: None,
        reason: reason.clone(),
        latency_ms: 0,
        ttft_ms: None,
        tps: 0.0,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost_usd: 0.0,
        model_output: String::new(),
        reasoning_content: None,
        tool_calls: None,
        finish_reason: None,
        trajectory_steps: None,
        error: Some(reason),
    }
}
