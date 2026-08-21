use eval_core::dataset::{Category, EvaluationType, TestCase};
use eval_core::evaluators::{
    CodeSandboxEvaluator, ExactMatchEvaluator, JsonSchemaEvaluator, RegexEvaluator, Evaluator,
};
use eval_core::metrics::EloCalculator;
use eval_core::model::{ModelResponse, TokenUsage};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

fn dummy_response(text: impl Into<String>) -> ModelResponse {
    ModelResponse {
        text: text.into(),
        reasoning_content: None,
        tool_calls: None,
        usage: TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 10,
            total_tokens: 20,
        },
        total_duration: Duration::from_millis(100),
        ttft: Some(Duration::from_millis(30)),
        tokens_per_second: 50.0,
        estimated_cost_usd: 0.0001,
        finish_reason: Some("stop".to_string()),
        raw_response: None,
    }
}

fn make_test_case(id: &str, eval_type: EvaluationType) -> TestCase {
    TestCase {
        id: id.to_string(),
        name: None,
        category: Category::Foundation,
        tags: vec![],
        prompt: "test prompt".to_string(),
        system_prompt: None,
        reference_answer: None,
        eval_type,
        criteria: None,
        schema: None,
        test_code: None,
        tools: None,
        max_turns: None,
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_exact_match_evaluator() {
    let eval = ExactMatchEvaluator::new(false, true);
    let mut tc = make_test_case("tc1", EvaluationType::ExactMatch);
    tc.reference_answer = Some("Paris".to_string());

    let resp_exact = dummy_response("paris");
    let res = eval.evaluate(&tc, &resp_exact).await.unwrap();
    assert!(res.passed);
    assert_eq!(res.score, 1.0);

    let resp_partial = dummy_response("The capital of France is Paris.");
    let res2 = eval.evaluate(&tc, &resp_partial).await.unwrap();
    assert!(res2.passed);
    assert_eq!(res2.score, 0.8);

    // Test math CoT with \boxed{\frac{1}{6}}
    let mut tc_math = make_test_case("tc_math", EvaluationType::ExactMatch);
    tc_math.reference_answer = Some("1/6".to_string());
    let math_cot = r#"we have two fair 6-sided dice. total possible outcomes: \( 6 \times 6 = 36 \). now, count the outcomes where the sum is 7: - (1, 6) - (2, 5) - (3, 4) - (4, 3) - (5, 2) - (6, 1) that’s 6 favorable outcomes. probability = \(\frac{6}{36} = \frac{1}{6}\). \[ \boxed{\frac{1}{6}} \]"#;
    let res_math = eval.evaluate(&tc_math, &dummy_response(math_cot)).await.unwrap();
    assert!(res_math.passed, "Math reasoning with boxed fraction should pass! Reason: {}", res_math.reason);
    assert_eq!(res_math.score, 1.0);

    let resp_fail = dummy_response("London");
    let res3 = eval.evaluate(&tc, &resp_fail).await.unwrap();
    assert!(!res3.passed);
}

#[tokio::test]
async fn test_regex_evaluator() {
    let eval = RegexEvaluator::new();
    let mut tc = make_test_case("tc2", EvaluationType::Regex);
    tc.criteria = Some(r"Answer:\s*\d+".to_string());

    let resp_pass = dummy_response("Here is my reasoning. Answer: 42");
    let res = eval.evaluate(&tc, &resp_pass).await.unwrap();
    assert!(res.passed);

    let resp_fail = dummy_response("Answer: forty two");
    let res2 = eval.evaluate(&tc, &resp_fail).await.unwrap();
    assert!(!res2.passed);
}

#[tokio::test]
async fn test_json_schema_evaluator() {
    let eval = JsonSchemaEvaluator::new();
    let mut tc = make_test_case("tc3", EvaluationType::JsonSchema);
    tc.schema = Some(json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "score": { "type": "number" }
        },
        "required": ["name", "score"]
    }));

    // Markdown block test
    let resp_md = dummy_response("```json\n{\"name\": \"Alice\", \"score\": 98.5}\n```");
    let res = eval.evaluate(&tc, &resp_md).await.unwrap();
    assert!(res.passed);

    // Missing field
    let resp_invalid = dummy_response("```json\n{\"name\": \"Alice\"}\n```");
    let res2 = eval.evaluate(&tc, &resp_invalid).await.unwrap();
    assert!(!res2.passed);
}

#[tokio::test]
async fn test_code_sandbox_evaluator() {
    let eval = CodeSandboxEvaluator::new("python", Duration::from_secs(5));
    let mut tc = make_test_case("tc4", EvaluationType::CodeExecution);
    tc.test_code = Some("assert add(2, 3) == 5\nassert add(-1, 1) == 0".to_string());

    let resp = dummy_response("```python\ndef add(a, b):\n    return a + b\n```");
    let res = eval.evaluate(&tc, &resp).await.unwrap();
    assert!(res.passed);

    let resp_fail = dummy_response("```python\ndef add(a, b):\n    return a * b\n```");
    let res2 = eval.evaluate(&tc, &resp_fail).await.unwrap();
    assert!(!res2.passed);
}

#[test]
fn test_elo_calculator() {
    let calc = EloCalculator::new(32.0, 1000.0);
    let (new_a, new_b) = calc.update_rating(1000.0, 1000.0, 1.0);
    assert_eq!(new_a, 1016.0);
    assert_eq!(new_b, 984.0);
}
