use crate::runner::AgentTrajectory;
use eval_core::dataset::TestCase;
use eval_core::evaluators::EvaluationResult;
use serde_json::{json, Value};
use std::collections::HashSet;

pub struct TrajectoryEvaluator;

impl TrajectoryEvaluator {
    pub fn evaluate(test_case: &TestCase, trajectory: &AgentTrajectory) -> EvaluationResult {
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
