use crate::env::SimulatedEnvironment;
use eval_core::model::{ChatMessage, ModelClient, ToolCall, ToolDefinition};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

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
        env: &dyn SimulatedEnvironment,
        max_turns: usize,
    ) -> anyhow::Result<AgentTrajectory> {
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

        for turn in 1..=max_turns {
            let turn_start = Instant::now();
            let response = match client.chat_complete(&messages, Some(tools)).await {
                Ok(resp) => resp,
                Err(e) => {
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
            };

            total_prompt_tokens += response.usage.prompt_tokens;
            total_completion_tokens += response.usage.completion_tokens;
            total_cost += response.estimated_cost_usd;

            let step_latency = turn_start.elapsed().as_millis() as u64;

            // Check if model decided to call tools
            if let Some(tool_calls) = response.tool_calls.clone() {
                if !tool_calls.is_empty() {
                    let mut tool_results = Vec::new();

                    // Push assistant message with tool calls
                    let mut assistant_msg = ChatMessage::assistant(&response.text);
                    assistant_msg.tool_calls = Some(tool_calls.clone());
                    messages.push(assistant_msg);

                    // Execute each tool
                    for tc in &tool_calls {
                        let res_str = match env.execute_tool(&tc.function.name, &tc.function.arguments) {
                            Ok(res) => res,
                            Err(err) => format!("Error executing tool '{}': {err}", tc.function.name),
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
