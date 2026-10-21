use eval_core::dataset::{Category, EvaluationType, TestCase};
use eval_core::evaluators::{
    CodeSandboxEvaluator, Evaluator, ExactMatchEvaluator, JsonSchemaEvaluator, LlmJudgeEvaluator,
    RegexEvaluator,
};
use eval_core::metrics::EloCalculator;
use eval_core::model::{MockClient, ModelResponse, TokenUsage};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
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
        difficulty: None,
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
    assert!(
        !res2.passed,
        "exact_match must not accept substring answers"
    );
    assert_eq!(res2.score, 0.0);
    let res_num = eval.evaluate(&tc, &dummy_response("4.0")).await.unwrap();
    let mut tc_num = tc.clone();
    tc_num.reference_answer = Some("4".to_string());
    assert!(
        eval.evaluate(&tc_num, &dummy_response("4.0"))
            .await
            .unwrap()
            .passed
    );
    let _ = res_num;

    // Test math CoT with \boxed{\frac{1}{6}}
    let mut tc_math = make_test_case("tc_math", EvaluationType::ExactMatch);
    tc_math.reference_answer = Some("1/6".to_string());
    let math_cot = r#"we have two fair 6-sided dice. total possible outcomes: \( 6 \times 6 = 36 \). now, count the outcomes where the sum is 7: - (1, 6) - (2, 5) - (3, 4) - (4, 3) - (5, 2) - (6, 1) that’s 6 favorable outcomes. probability = \(\frac{6}{36} = \frac{1}{6}\). \[ \boxed{\frac{1}{6}} \]"#;
    let res_math = eval
        .evaluate(&tc_math, &dummy_response(math_cot))
        .await
        .unwrap();
    assert!(
        res_math.passed,
        "Math reasoning with boxed fraction should pass! Reason: {}",
        res_math.reason
    );
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

#[tokio::test]
async fn test_regex_negative_constraint() {
    let eval = RegexEvaluator::new();
    let mut tc = make_test_case("tc_neg", EvaluationType::Regex);
    tc.criteria = Some("(?i)(fruit.*sweet|sweet.*fruit)".to_string());
    tc.metadata
        .insert("must_not_match".to_string(), json!(["(?i)banana"]));
    let pass = eval
        .evaluate(&tc, &dummy_response("An apple is a sweet fruit."))
        .await
        .unwrap();
    assert!(pass.passed);
    let fail = eval
        .evaluate(
            &tc,
            &dummy_response("An apple is a sweet fruit; banana is also a fruit."),
        )
        .await
        .unwrap();
    assert!(!fail.passed);
}

#[tokio::test]
async fn test_llm_judge_non_json_fails_closed() {
    let judge: Arc<dyn eval_core::model::ModelClient> =
        Arc::new(MockClient::new("judge", "judge").with_default_response("I think it is correct."));
    let eval = LlmJudgeEvaluator::new(judge);
    let mut tc = make_test_case("tc_judge", EvaluationType::LlmJudge);
    tc.criteria = Some("answer must be 4".to_string());
    let res = eval
        .evaluate(&tc, &dummy_response("The answer is 4"))
        .await
        .unwrap();
    assert!(!res.passed);
    assert!(res.reason.contains("non-JSON"));
}

#[tokio::test]
async fn test_humaneval_style_harness() {
    let eval = CodeSandboxEvaluator::default();
    let mut tc = make_test_case("humaneval_harness", EvaluationType::CodeExecution);
    tc.test_code = Some(
        "def check(candidate):\n    assert candidate(2) == 4\n    assert candidate(3) == 6\ncheck(solve)\n"
            .to_string(),
    );
    let res = eval
        .evaluate(
            &tc,
            &dummy_response("```python\ndef solve(x):\n    return x * 2\n```"),
        )
        .await
        .unwrap();
    assert!(res.passed, "reason={}", res.reason);
}

#[tokio::test]
async fn test_mbpp_style_harness() {
    let eval = CodeSandboxEvaluator::default();
    let mut tc = make_test_case("mbpp_harness", EvaluationType::CodeExecution);
    tc.test_code = Some("assert add(1, 2) == 3\nassert add(-1, 1) == 0\n".to_string());
    let res = eval
        .evaluate(
            &tc,
            &dummy_response("```python\ndef add(a, b):\n    return a + b\n```"),
        )
        .await
        .unwrap();
    assert!(res.passed, "reason={}", res.reason);
}
