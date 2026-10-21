use crate::env::SimulatedEnvironment;
use eval_core::model::{
    extract_fallback_tool_calls, ChatMessage, ModelClient, ToolCall, ToolDefinition,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_TOOL_CALLS_PER_TURN: usize = 8;
const MAX_TOTAL_TOOL_CALLS: usize = 64;
const MAX_TOOL_OUTPUT_CHARS: usize = 64_000;
const MAX_TOTAL_TOKENS: u32 = 2_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub turn: usize,
    pub model_thought: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<(String, String)>>, // (tool_call_id, result_string)
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrajectory {
    pub task_id: String,
    pub steps: Vec<AgentStep>,
    pub final_answer: Option<String>,
    pub total_turns: usize,
    pub completed: bool,
    pub total_duration: Duration,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub estimated_cost_usd: f64,
}

pub struct AgentRunner;

impl AgentRunner {
    pub async fn run(
        client: &dyn ModelClient,
        system_prompt: Option<&str>,
        user_prompt: &str,
        tools: &[ToolDefinition],
        env: Arc<dyn SimulatedEnvironment>,
        max_turns: usize,
    ) -> anyhow::Result<AgentTrajectory> {
        let max_turns = max_turns.clamp(1, 64);
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(user_prompt));

        let start_time = Instant::now();
        let mut steps = Vec::new();
        let mut final_answer = None;
        let mut completed = false;

        let mut total_prompt_tokens = 0u32;
        let mut total_completion_tokens = 0u32;
        let mut total_cost = 0.0f64;
        let mut total_tool_calls = 0usize;

        for turn in 1..=max_turns {
            let turn_start = Instant::now();
            let response = match tokio::time::timeout(
                std::time::Duration::from_secs(90),
                client.chat_complete(&messages, Some(tools)),
            )
            .await
            {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => {
                    steps.push(AgentStep {
                        turn,
                        model_thought: String::new(),
                        tool_calls: None,
                        tool_results: None,
                        latency_ms: turn_start.elapsed().as_millis() as u64,
                        error: Some(format!("Model call failed: {e}")),
                    });
                    break;
                }
                Err(_) => {
                    steps.push(AgentStep {
                        turn,
                        model_thought: String::new(),
                        tool_calls: None,
                        tool_results: None,
                        latency_ms: turn_start.elapsed().as_millis() as u64,
                        error: Some("Step LLM call timed out after 90s".to_string()),
                    });
                    break;
                }
            };

            total_prompt_tokens = total_prompt_tokens.saturating_add(response.usage.prompt_tokens);
            total_completion_tokens =
                total_completion_tokens.saturating_add(response.usage.completion_tokens);
            total_cost += response.estimated_cost_usd;
            if total_prompt_tokens.saturating_add(total_completion_tokens) > MAX_TOTAL_TOKENS {
                steps.push(AgentStep {
                    turn,
                    model_thought: String::new(),
                    tool_calls: None,
                    tool_results: None,
                    latency_ms: turn_start.elapsed().as_millis() as u64,
                    error: Some(format!(
                        "Token budget exceeded ({} > {})",
                        total_prompt_tokens.saturating_add(total_completion_tokens),
                        MAX_TOTAL_TOKENS
                    )),
                });
                break;
            }

            let step_latency = turn_start.elapsed().as_millis() as u64;

            // Check if model decided to call tools (either structured API tool_calls or fallback text markup)
            let tool_calls = response
                .tool_calls
                .clone()
                .filter(|tc| !tc.is_empty())
                .or_else(|| extract_fallback_tool_calls(&response.text));

            if let Some(mut tool_calls) = tool_calls {
                if tool_calls.len() > MAX_TOOL_CALLS_PER_TURN {
                    tool_calls.truncate(MAX_TOOL_CALLS_PER_TURN);
                }
                if total_tool_calls.saturating_add(tool_calls.len()) > MAX_TOTAL_TOOL_CALLS {
                    steps.push(AgentStep {
                        turn,
                        model_thought: response.text.clone(),
                        tool_calls: None,
                        tool_results: None,
                        latency_ms: step_latency,
                        error: Some(format!(
                            "Tool-call budget exceeded ({} > {})",
                            total_tool_calls, MAX_TOTAL_TOOL_CALLS
                        )),
                    });
                    break;
                }
                if !tool_calls.is_empty() {
                    total_tool_calls += tool_calls.len();
                    let mut tool_results = Vec::new();

                    // Push assistant message with tool calls
                    let mut assistant_msg = ChatMessage::assistant(&response.text);
                    assistant_msg.tool_calls = Some(tool_calls.clone());
                    messages.push(assistant_msg);

                    // Execute each tool
                    for tc in &tool_calls {
                        let env_for_tool = Arc::clone(&env);
                        let tool_name = tc.function.name.clone();
                        let tool_args = tc.function.arguments.clone();
                        let res_str = match tokio::task::spawn_blocking(move || {
                            env_for_tool.execute_tool(&tool_name, &tool_args)
                        })
                        .await
                        {
                            Ok(Ok(res)) => res,
                            Ok(Err(err)) => {
                                format!("Error executing tool '{}': {err}", tc.function.name)
                            }
                            Err(join_err) => format!(
                                "Error executing tool '{}': worker panicked: {join_err}",
                                tc.function.name
                            ),
                        };

                        let res_str = if res_str.chars().count() > MAX_TOOL_OUTPUT_CHARS {
                            let truncated: String =
                                res_str.chars().take(MAX_TOOL_OUTPUT_CHARS).collect();
                            format!("{truncated}\n[tool output truncated]")
                        } else {
                            res_str
                        };
                        tool_results.push((tc.id.clone(), res_str.clone()));

                        // Append tool response
                        messages.push(ChatMessage::tool_response(
                            &tc.id,
                            &tc.function.name,
                            res_str,
                        ));
                    }

                    steps.push(AgentStep {
                        turn,
                        model_thought: response.text,
                        tool_calls: Some(tool_calls),
                        tool_results: Some(tool_results),
                        latency_ms: step_latency,
                        error: None,
                    });

                    // If we just executed the final allowed turn, prompt the model to formulate its complete final answer
                    if turn == max_turns {
                        let final_start = Instant::now();
                        messages.push(ChatMessage::user(
                            eval_core::prompts::AGENT_FINAL_SYNTHESIS.template,
                        ));

                        match tokio::time::timeout(
                            std::time::Duration::from_secs(90),
                            client.chat_complete(&messages, None),
                        )
                        .await
                        {
                            Ok(Ok(final_resp)) => {
                                total_prompt_tokens += final_resp.usage.prompt_tokens;
                                total_completion_tokens += final_resp.usage.completion_tokens;
                                total_cost += final_resp.estimated_cost_usd;

                                final_answer = Some(final_resp.text.clone());
                                completed = true;

                                steps.push(AgentStep {
                                    turn: turn + 1,
                                    model_thought: final_resp.text,
                                    tool_calls: None,
                                    tool_results: None,
                                    latency_ms: final_start.elapsed().as_millis() as u64,
                                    error: None,
                                });
                            }
                            Ok(Err(e)) => {
                                steps.push(AgentStep {
                                    turn: turn + 1,
                                    model_thought: String::new(),
                                    tool_calls: None,
                                    tool_results: None,
                                    latency_ms: final_start.elapsed().as_millis() as u64,
                                    error: Some(format!("Final synthesis failed: {e}")),
                                });
                            }
                            Err(_) => {
                                steps.push(AgentStep {
                                    turn: turn + 1,
                                    model_thought: String::new(),
                                    tool_calls: None,
                                    tool_results: None,
                                    latency_ms: final_start.elapsed().as_millis() as u64,
                                    error: Some("Final synthesis timed out after 90s".to_string()),
                                });
                            }
                        }
                    }

                    continue;
                }
            }

            // Model finished without tool calls -> this is the final answer
            final_answer = Some(response.text.clone());
            completed = true;

            steps.push(AgentStep {
                turn,
                model_thought: response.text,
                tool_calls: None,
                tool_results: None,
                latency_ms: step_latency,
                error: None,
            });

            break;
        }

        // Fallback: if final_answer is still None, take the latest non-empty thought
        if final_answer.is_none() {
            for step in steps.iter().rev() {
                if !step.model_thought.trim().is_empty() {
                    final_answer = Some(step.model_thought.clone());
                    break;
                }
            }
        }

        Ok(AgentTrajectory {
            task_id: user_prompt.to_string(),
            total_turns: steps.len(),
            steps,
            final_answer,
            completed,
            total_duration: start_time.elapsed(),
            prompt_tokens: total_prompt_tokens,
            completion_tokens: total_completion_tokens,
            estimated_cost_usd: total_cost,
        })
    }
}
