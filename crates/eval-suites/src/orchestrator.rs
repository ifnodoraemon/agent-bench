use eval_agent::{get_pi_tools, AgentRunner, TrajectoryEvaluator, WorkspaceEnv};
use eval_core::dataset::{Dataset, EvaluationType, TestCase};
use eval_core::evaluators::{
    CodeSandboxEvaluator, ExactMatchEvaluator, JsonSchemaEvaluator, LlmJudgeEvaluator,
    RegexEvaluator,
};
use eval_core::metrics::{CaseResult, ModelBenchmarkSummary};
use eval_core::model::{ChatMessage, ModelClient};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;

pub struct BenchmarkOrchestrator {
    concurrency: usize,
    judge_client: Option<Arc<dyn ModelClient>>,
}

impl BenchmarkOrchestrator {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency: concurrency.max(1),
            judge_client: None,
        }
    }

    pub fn with_judge(mut self, judge: Arc<dyn ModelClient>) -> Self {
        self.judge_client = Some(judge);
        self
    }

    pub async fn run_benchmark(
        &self,
        client: Arc<dyn ModelClient>,
        dataset: &Dataset,
    ) -> anyhow::Result<ModelBenchmarkSummary> {
        let total_cases = dataset.test_cases.len();
        let pb = ProgressBar::new(total_cases as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!("Evaluating '{}'", client.config().model_name));

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut tasks = Vec::new();

        for test_case in dataset.test_cases.clone() {
            let client_clone = Arc::clone(&client);
            let sem_clone = Arc::clone(&semaphore);
            let judge_clone = self.judge_client.clone();
            let pb_clone = pb.clone();

            let task = tokio::spawn(async move {
                let _permit = sem_clone.acquire().await.unwrap();
                let res = Self::execute_single_case(client_clone, judge_clone, test_case).await;
                pb_clone.inc(1);
                res
            });

            tasks.push(task);
        }

        let mut case_results = Vec::new();
        for task in tasks {
            let res = task.await??;
            case_results.push(res);
        }

        pb.finish_with_message(format!(
            "Completed benchmark for '{}'",
            client.config().model_name
        ));

        let summary = ModelBenchmarkSummary::compute(
            client.config().id.clone(),
            client.config().model_name.clone(),
            case_results,
        );

        Ok(summary)
    }

    async fn execute_single_case(
        client: Arc<dyn ModelClient>,
        judge_client: Option<Arc<dyn ModelClient>>,
        test_case: TestCase,
    ) -> anyhow::Result<CaseResult> {
        let start_time = Instant::now();

        if test_case.eval_type == EvaluationType::AgentTrajectory {
            // Initialize isolated workspace execution environment
            let workspace = WorkspaceEnv::new()?;

            // Seed initial files into workspace if defined in metadata
            if let Some(setup_val) = test_case.metadata.get("setup_files") {
                if let Ok(setup_map) = serde_json::from_value::<HashMap<String, String>>(setup_val.clone()) {
                    workspace.setup_from_files(&setup_map)?;
                }
            }

            // Default to Pi primitive tools (bash, read_file, write_file, edit_file, grep_search)
            let tools = test_case.tools.clone().unwrap_or_else(get_pi_tools);
            let max_turns = test_case.max_turns.unwrap_or(8);

            let trajectory = AgentRunner::run(
                client.as_ref(),
                test_case.system_prompt.as_deref(),
                &test_case.prompt,
                &tools,
                &workspace,
                max_turns,
            )
            .await?;

            let mut eval_res = TrajectoryEvaluator::evaluate_with_judge(
                &test_case,
                &trajectory,
                judge_client.as_ref(),
            )
            .await;

            // If a post-task verification command is defined in test_code or metadata
            let verify_cmd = test_case.test_code.as_deref().or_else(|| {
                test_case.metadata.get("verify_cmd").and_then(|v| v.as_str())
            });

            if let Some(cmd) = verify_cmd {
                match workspace.run_verification(cmd) {
                    Ok((passed, verify_output)) => {
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
                    Err(e) => {
                        eval_res.passed = false;
                        eval_res.score = 0.0;
                        eval_res.reason = format!("Verification command error: {e}");
                    }
                }
            }

            let latency_ms = start_time.elapsed().as_millis() as u64;
            let final_output = trajectory
                .final_answer
                .clone()
                .unwrap_or_else(|| "[No final text returned]".to_string());

            return Ok(CaseResult {
                test_case_id: test_case.id,
                test_case_name: test_case.name,
                category: test_case.category,
                passed: eval_res.passed,
                score: eval_res.score,
                reason: eval_res.reason,
                latency_ms,
                ttft_ms: None,
                tps: 0.0,
                prompt_tokens: trajectory.prompt_tokens,
                completion_tokens: trajectory.completion_tokens,
                cost_usd: trajectory.estimated_cost_usd,
                model_output: final_output,
                error: None,
            });
        }

        // Standard single-turn evaluation
        let mut messages = Vec::new();
        if let Some(ref sys) = test_case.system_prompt {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(&test_case.prompt));

        let model_resp = match client
            .chat_complete(&messages, test_case.tools.as_deref())
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                return Ok(CaseResult {
                    test_case_id: test_case.id,
                    test_case_name: test_case.name,
                    category: test_case.category,
                    passed: false,
                    score: 0.0,
                    reason: format!("Model invocation error: {e}"),
                    latency_ms: start_time.elapsed().as_millis() as u64,
                    ttft_ms: None,
                    tps: 0.0,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    cost_usd: 0.0,
                    model_output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        let latency_ms = model_resp.total_duration.as_millis() as u64;
        let ttft_ms = model_resp.ttft.map(|t| t.as_millis() as u64);

        let eval_res = match test_case.eval_type {
            EvaluationType::ExactMatch => {
                let exact = ExactMatchEvaluator::default();
                eval_core::evaluators::Evaluator::evaluate(&exact, &test_case, &model_resp).await?
            }
            EvaluationType::Regex => {
                let regex_eval = RegexEvaluator::default();
                eval_core::evaluators::Evaluator::evaluate(&regex_eval, &test_case, &model_resp).await?
            }
            EvaluationType::JsonSchema => {
                let schema_eval = JsonSchemaEvaluator::default();
                eval_core::evaluators::Evaluator::evaluate(&schema_eval, &test_case, &model_resp).await?
            }
            EvaluationType::CodeExecution => {
                let code_eval = CodeSandboxEvaluator::default();
                eval_core::evaluators::Evaluator::evaluate(&code_eval, &test_case, &model_resp).await?
            }
            EvaluationType::LlmJudge => {
                if let Some(ref judge) = judge_client {
                    let judge_eval = LlmJudgeEvaluator::new(judge.clone());
                    eval_core::evaluators::Evaluator::evaluate(&judge_eval, &test_case, &model_resp).await?
                } else {
                    let exact = ExactMatchEvaluator::default();
                    eval_core::evaluators::Evaluator::evaluate(&exact, &test_case, &model_resp).await?
                }
            }
            EvaluationType::AgentTrajectory => unreachable!(),
        };

        Ok(CaseResult {
            test_case_id: test_case.id,
            test_case_name: test_case.name,
            category: test_case.category,
            passed: eval_res.passed,
            score: eval_res.score,
            reason: eval_res.reason,
            latency_ms,
            ttft_ms,
            tps: model_resp.tokens_per_second,
            prompt_tokens: model_resp.usage.prompt_tokens,
            completion_tokens: model_resp.usage.completion_tokens,
            cost_usd: model_resp.estimated_cost_usd,
            model_output: model_resp.text,
            error: None,
        })
    }
}
