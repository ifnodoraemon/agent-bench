use eval_agent::{
    runner::{AgentStep, AgentTrajectory},
    RealSubprocessBashEnv, SimulatedEnvironment, TrajectoryEvaluator,
};
use eval_core::dataset::{Category, EvaluationType, TestCase};
use eval_core::model::{FunctionCall, ToolCall};
use std::collections::HashMap;

#[test]
fn test_real_subprocess_bash_sandbox() {
    let env = RealSubprocessBashEnv::new().unwrap();

    // 1. Test real bash execution in sandbox
    let res = env
        .execute_tool("bash", r#"{"command": "echo 'Hello from real sandbox' > test.txt && cat test.txt"}"#)
        .unwrap();
    assert!(res.contains("Hello from real sandbox"));

    // 2. Verify file was created in sandbox dir
    let file_content = env.read_file("test.txt").unwrap();
    assert_eq!(file_content.trim(), "Hello from real sandbox");
}

#[test]
fn test_trajectory_evaluator_5_dimensional_scoring() {
    let tool_call = ToolCall {
        id: "call_123".to_string(),
        tool_type: "function".to_string(),
        function: FunctionCall {
            name: "bash".to_string(),
            arguments: r#"{"command": "python3 -m unittest tests/test_math.py"}"#.to_string(),
        },
    };

    let step = AgentStep {
        turn: 1,
        model_thought: "I will run the unittest".to_string(),
        tool_calls: Some(vec![tool_call]),
        tool_results: Some(vec![("call_123".to_string(), "OK (Ran 2 tests)".to_string())]),
        latency_ms: 50,
        error: None,
    };

    let trajectory = AgentTrajectory {
        task_id: "task_001".to_string(),
        steps: vec![step],
        final_answer: Some("All tests passed with 8080".to_string()),
        total_turns: 1,
        completed: true,
        total_duration: std::time::Duration::from_millis(100),
        prompt_tokens: 50,
        completion_tokens: 30,
        estimated_cost_usd: 0.0001,
    };

    let tc = TestCase {
        id: "test_01".to_string(),
        name: None,
        category: Category::Agent,
        tags: vec![],
        prompt: "Please check the port".to_string(),
        system_prompt: None,
        reference_answer: Some("8080".to_string()),
        eval_type: EvaluationType::AgentTrajectory,
        criteria: None,
        schema: None,
        test_code: None,
        tools: None,
        max_turns: Some(5),
        metadata: HashMap::new(),
    };

    let eval_res = TrajectoryEvaluator::evaluate(&tc, &trajectory);
    assert!(eval_res.passed);
    assert!(eval_res.score >= 0.8);
}
