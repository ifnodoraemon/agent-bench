pub mod dataset;
pub mod evaluators;
pub mod metrics;
pub mod model;
pub mod prompts;
pub mod reporter;
pub mod verifier;

pub use dataset::{Category, Dataset, DatasetLoader, EvaluationType, TestCase};
pub use evaluators::{
    CodeSandboxEvaluator, EvaluationResult, Evaluator, EvaluatorRegistry, ExactMatchEvaluator,
    JsonSchemaEvaluator, LlmJudgeEvaluator, RegexEvaluator,
};
pub use metrics::{CaseResult, CategorySummary, EloCalculator, ModelBenchmarkSummary};
pub use model::{
    AnthropicClient, ApiProtocol, ChatMessage, FunctionCall, FunctionDefinition, GeminiClient,
    ModelClient, ModelConfig, ModelResponse, OpenAICompatibleClient, OpenAIResponsesClient, Role,
    TokenUsage, ToolCall, ToolDefinition,
};
pub use prompts::{PromptCatalog, PromptTemplate};
pub use verifier::{
    evaluate_baseline_drift, get_all_baselines, get_official_baseline, resolve_canonical_id,
    BaselineDriftReport, BenchmarkMetric, DriftMetricComparison, DriftStatus, ModelVerifier,
    OfficialModelBaseline, ProbeCategory, ProbeDefinition, ProbeResult, VerificationReport,
    OFFICIAL_BASELINES, STANDARD_PROBES,
};
