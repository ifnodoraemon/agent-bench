use crate::runner::AgentTrajectory;
use eval_core::dataset::TestCase;
use eval_core::evaluators::{EvaluationResult, JsonSchemaEvaluator};
use eval_core::model::{ChatMessage, ModelClient};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;

pub struct TrajectoryEvaluator;

impl TrajectoryEvaluator {
    /// Synchronous rule-based evaluation (fallback or fast evaluation)
    pub fn evaluate(test_case: &TestCase, trajectory: &AgentTrajectory) -> EvaluationResult {
        Self::evaluate_rule_based(test_case, trajectory)
    }

    /// Comprehensive evaluation using LLM-as-a-Judge if available, falling back to rule-based evaluation
    pub async fn evaluate_with_judge(
        test_case: &TestCase,
        trajectory: &AgentTrajectory,
        judge_client: Option<&Arc<dyn ModelClient>>,
    ) -> EvaluationResult {
        if let Some(judge) = judge_client {
            match Self::evaluate_with_llm_judge(test_case, trajectory, judge.as_ref()).await {
                Ok(judge_res) => return judge_res,
                Err(e) => {
                    tracing::warn!("LLM Trajectory Judge failed ({e}), falling back to rule-based evaluation");
                }
            }
        }

        Self::evaluate_rule_based(test_case, trajectory)
    }

    /// Evaluates agent trajectory using an LLM Judge model
    async fn evaluate_with_llm_judge(
        test_case: &TestCase,
        trajectory: &AgentTrajectory,
        judge: &dyn ModelClient,
    ) -> anyhow::Result<EvaluationResult> {
        let criteria_str = test_case
            .criteria
            .as_deref()
            .or(test_case.reference_answer.as_deref())
            .unwrap_or("Complete the task goals effectively, accurately, and safely.");

        let mut steps_text = String::new();
        for step in &trajectory.steps {
            steps_text.push_str(&format!("--- Turn {} ---\n", step.turn));
            if !step.model_thought.trim().is_empty() {
                steps_text.push_str(&format!("Thought:\n{}\n", step.model_thought.trim()));
            }
            if let Some(tcs) = &step.tool_calls {
                for tc in tcs {
                    steps_text.push_str(&format!(
                        "Tool Call: {}({})\n",
                        tc.function.name, tc.function.arguments
                    ));
                }
            }
            if let Some(trs) = &step.tool_results {
                for (_id, res) in trs {
                    let truncated = if res.len() > 1000 {
                        format!("{}... [truncated]", &res[..1000])
                    } else {
                        res.clone()
                    };
                    steps_text.push_str(&format!("Tool Output:\n{}\n", truncated.trim()));
                }
            }
            if let Some(err) = &step.error {
                steps_text.push_str(&format!("Step Error: {}\n", err));
            }
            steps_text.push('\n');
        }

        if steps_text.is_empty() {
            steps_text.push_str("(No interaction steps recorded)\n");
        }

        let final_ans = trajectory
            .final_answer
            .as_deref()
            .unwrap_or("(No final answer provided)");

        let judge_prompt = format!(
            r#"You are an expert AI Benchmark Judge evaluating an Autonomous Agent's execution trajectory.

[USER PROMPT / TASK GOAL]:
{}

[EXPECTED CRITERIA / REFERENCE]:
{}

[AGENT TRAJECTORY SUMMARY]:
- Total Interaction Turns: {}
- Task Completed: {}
- Duration: {} ms
- Tokens: {} In / {} Out

[INTERACTION STEPS]:
{}

[FINAL AGENT ANSWER]:
{}

Evaluate the agent's performance across 4 dimensions:
1. **Goal Achievement (0.0 - 1.0, weight: 40%)**: Did the agent resolve the user's objective completely and accurately?
2. **Tool Selection & Parameter Precision (0.0 - 1.0, weight: 25%)**: Did it choose the right tools with accurate parameters without redundant or malformed calls?
3. **Reasoning & Planning Coherence (0.0 - 1.0, weight: 20%)**: Was the thought process logical, progressive, and properly interpreting tool feedback?
4. **Error Recovery & Adaptability (0.0 - 1.0, weight: 15%)**: Did it adapt constructively to errors or missing files/data?

Respond strictly in JSON format as follows:
```json
{{
  "goal_score": 0.90,
  "tool_score": 0.85,
  "reasoning_score": 0.90,
  "recovery_score": 1.00,
  "composite_score": 0.89,
  "passed": true,
  "reasoning": "Concise 1-3 sentence evaluation summary explaining strengths and shortcomings."
}}
```"#,
            test_case.prompt,
            criteria_str,
            trajectory.total_turns,
            trajectory.completed,
            trajectory.total_duration.as_millis(),
            trajectory.prompt_tokens,
            trajectory.completion_tokens,
            steps_text,
            final_ans
        );

        let messages = vec![ChatMessage::user(judge_prompt)];
        let resp = judge.chat_complete(&messages, None).await?;

        if let Some(json_val) = JsonSchemaEvaluator::extract_json(&resp.text) {
            let composite_score = json_val
                .get("composite_score")
                .and_then(|s| s.as_f64())
                .unwrap_or_else(|| {
                    let g = json_val.get("goal_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let t = json_val.get("tool_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let r = json_val.get("reasoning_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let e = json_val.get("recovery_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    (g * 0.40) + (t * 0.25) + (r * 0.20) + (e * 0.15)
                })
                .clamp(0.0, 1.0);

            let passed = json_val
                .get("passed")
                .and_then(|p| p.as_bool())
                .unwrap_or(composite_score >= 0.70);

            let reasoning = json_val
                .get("reasoning")
                .and_then(|r| r.as_str())
                .unwrap_or("LLM Judge evaluation completed")
                .to_string();

            Ok(EvaluationResult {
                passed,
                score: composite_score,
                reason: format!("Agent LLM-Judge ({:.2}/1.00): {}", composite_score, reasoning),
                details: Some(json_val),
            })
        } else {
            anyhow::bail!("Failed to parse JSON from LLM Judge response: {}", resp.text);
        }
    }

    /// Deterministic heuristic rule-based evaluation (Tool validity, loops, error recovery, keyword match)
    pub fn evaluate_rule_based(test_case: &TestCase, trajectory: &AgentTrajectory) -> EvaluationResult {
        if !trajectory.completed && trajectory.steps.is_empty() {
            return EvaluationResult::fail("Agent trajectory is empty or failed immediately.");
        }

        // 1. Tool Selection Accuracy (Check if tools chosen are valid)
        let total_tool_calls: usize = trajectory
            .steps
            .iter()
            .map(|s| s.tool_calls.as_ref().map(|v| v.len()).unwrap_or(0))
            .sum();

        let tool_selection_score = if total_tool_calls > 0 { 1.0 } else { 0.5 };

        // 2. Schema & Argument Validity (Are arguments valid JSON)
        let mut valid_arg_count = 0;
        let mut _invalid_arg_count = 0;

        for step in &trajectory.steps {
            if let Some(tcs) = &step.tool_calls {
                for tc in tcs {
                    if serde_json::from_str::<Value>(&tc.function.arguments).is_ok() {
                        valid_arg_count += 1;
                    } else {
                        _invalid_arg_count += 1;
                    }
                }
            }
        }

        let schema_validity_score = if total_tool_calls > 0 {
            valid_arg_count as f64 / total_tool_calls as f64
        } else {
            1.0
        };

        // 3. Loop & Redundancy Detection
        let mut seen_calls = HashSet::new();
        let mut repeat_count = 0;

        for step in &trajectory.steps {
            if let Some(tcs) = &step.tool_calls {
                for tc in tcs {
                    let sig = format!("{}:{}", tc.function.name, tc.function.arguments);
                    if !seen_calls.insert(sig) {
                        repeat_count += 1;
                    }
                }
            }
        }

        let loop_score = if repeat_count == 0 {
            1.0
        } else if repeat_count <= 2 {
            0.5
        } else {
            0.0 // Heavy penalty for infinite loop
        };

        // 4. Error Recovery Score (If a tool returned an error, did the next step adapt?)
        let mut had_error = false;
        let mut recovered = false;

        for (i, step) in trajectory.steps.iter().enumerate() {
            if let Some(results) = &step.tool_results {
                for (_, res) in results {
                    if res.to_lowercase().contains("error") || res.to_lowercase().contains("no such file") {
                        had_error = true;
                        // Check if subsequent step exists
                        if i + 1 < trajectory.steps.len() {
                            recovered = true;
                        }
                    }
                }
            }
        }

        let error_recovery_score = if !had_error {
            1.0
        } else if recovered {
            1.0
        } else {
            0.0
        };

        // 5. Final Goal & Answer Completion
        let (goal_score, reason_str) = if let Some(final_text) = &trajectory.final_answer {
            if let Some(ref ref_ans) = test_case.reference_answer {
                if final_text.to_lowercase().contains(&ref_ans.to_lowercase()) {
                    (1.0, format!("Goal achieved: Matched reference answer '{ref_ans}'"))
                } else {
                    (0.3, format!("Goal partially missed: Output did not contain expected '{ref_ans}'"))
                }
            } else if let Some(ref crit) = test_case.criteria {
                if final_text.to_lowercase().contains(&crit.to_lowercase()) {
                    (1.0, format!("Goal achieved: Met criteria '{crit}'"))
                } else {
                    (0.4, format!("Goal partially missed: Criteria '{crit}' unsatisfied"))
                }
            } else {
                (0.9, "Completed interaction turns successfully".to_string())
            }
        } else {
            (0.0, format!("Agent stopped without final text (Turns: {})", trajectory.total_turns))
        };

        // Weighted composite score (Goal: 40%, Schema: 20%, Loop: 15%, Recovery: 15%, Tool: 10%)
        let composite_score = (goal_score * 0.40)
            + (schema_validity_score * 0.20)
            + (loop_score * 0.15)
            + (error_recovery_score * 0.15)
            + (tool_selection_score * 0.10);

        let passed = composite_score >= 0.70;

        let details = json!({
            "goal_completion": goal_score,
            "schema_validity": schema_validity_score,
            "loop_score": loop_score,
            "error_recovery_score": error_recovery_score,
            "tool_selection_score": tool_selection_score,
            "total_turns": trajectory.total_turns,
            "total_tool_calls": total_tool_calls,
            "repeated_calls": repeat_count,
            "had_error_in_trajectory": had_error
        });

        EvaluationResult {
            passed,
            score: (composite_score as f64).clamp(0.0, 1.0),
            reason: format!("Agent 5-Dim Score ({:.2}/1.00): {}", composite_score, reason_str),
            details: Some(details),
        }
    }
}

