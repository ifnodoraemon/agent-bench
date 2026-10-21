use crate::model::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Foundation,
    Agent,
    Safety,
    Swe,
    Devops,
    Security,
    DataAnalyst,
    MathLogic,
    Multilingual,
    LongContext,
    Instruction,
    StructuredOutput,
    Medical,
    Legal,
    Finance,
    Science,
    Humanities,
    Performance,
    #[serde(untagged)]
    Custom(String),
}

impl Category {
    pub fn as_str(&self) -> &str {
        match self {
            Category::Foundation => "foundation",
            Category::Agent => "agent",
            Category::Safety => "safety",
            Category::Swe => "swe",
            Category::Devops => "devops",
            Category::Security => "security",
            Category::DataAnalyst => "data_analyst",
            Category::MathLogic => "math_logic",
            Category::Multilingual => "multilingual",
            Category::LongContext => "long_context",
            Category::Instruction => "instruction",
            Category::StructuredOutput => "structured_output",
            Category::Medical => "medical",
            Category::Legal => "legal",
            Category::Finance => "finance",
            Category::Science => "science",
            Category::Humanities => "humanities",
            Category::Performance => "performance",
            Category::Custom(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Category::Foundation => "Foundation (基础基准)",
            Category::Agent => "Agent Core (智能体核心)",
            Category::Safety => "Safety & Defense (安全防御)",
            Category::Swe => "Coding & SWE (软件工程)",
            Category::Devops => "DevOps & Cloud (云原生运维)",
            Category::Security => "Security & Audit (网络安全攻防)",
            Category::DataAnalyst => "Data & SQL (数据分析与BI)",
            Category::MathLogic => "Math & Logic (数理逻辑推理)",
            Category::Multilingual => "Multilingual (多语言与NLP)",
            Category::LongContext => "Long Context (长文本与大海捞针)",
            Category::Instruction => "Instruction (指令遵循与约束)",
            Category::StructuredOutput => "Structured Output (结构化输出)",
            Category::Medical => "Medicine & Healthcare (医疗健康)",
            Category::Legal => "Law & Jurisprudence (法律法规)",
            Category::Finance => "Finance & Economics (金融商科)",
            Category::Science => "Natural Sciences (自然科学)",
            Category::Humanities => "Humanities & Philosophy (人文社科)",
            Category::Performance => "Performance (吞吐与延迟)",
            Category::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationType {
    ExactMatch,
    Regex,
    JsonSchema,
    CodeExecution,
    LlmJudge,
    AgentTrajectory,
    Numeric,
    MultipleChoice,
    ToolCall,
}

impl std::str::FromStr for EvaluationType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "exact_match" | "exact" => Ok(EvaluationType::ExactMatch),
            "regex" => Ok(EvaluationType::Regex),
            "json_schema" | "schema" | "json" => Ok(EvaluationType::JsonSchema),
            "code_execution" | "code" | "sandbox" => Ok(EvaluationType::CodeExecution),
            "llm_judge" | "judge" => Ok(EvaluationType::LlmJudge),
            "numeric" | "number" => Ok(EvaluationType::Numeric),
            "multiple_choice" | "multiplechoice" | "mcq" | "mc" => {
                Ok(EvaluationType::MultipleChoice)
            }
            "tool_call" | "toolcall" | "function_call" | "functioncall" => {
                Ok(EvaluationType::ToolCall)
            }
            "agent_trajectory" | "agent" | "trajectory" | "tools" => {
                Ok(EvaluationType::AgentTrajectory)
            }
            other => Err(anyhow::anyhow!("Unknown EvaluationType: {other}")),
        }
    }
}

impl EvaluationType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub category: Category,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub prompt: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(
        default,
        alias = "expected_output",
        alias = "expected",
        alias = "answer"
    )]
    pub reference_answer: Option<String>,
    pub eval_type: EvaluationType,
    #[serde(default, alias = "rubric", alias = "judge_criteria")]
    pub criteria: Option<String>,
    #[serde(default, alias = "json_schema")]
    pub schema: Option<serde_json::Value>,
    #[serde(default, alias = "code", alias = "script")]
    pub test_code: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(default)]
    pub max_turns: Option<usize>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TestCase {
    /// Calculate adaptive timeout in seconds based on difficulty, eval_type, and max_turns
    pub fn timeout_seconds(&self) -> u64 {
        // 1. Explicit metadata timeout
        if let Some(t) = self
            .metadata
            .get("timeout_secs")
            .or_else(|| self.metadata.get("timeout"))
            .and_then(|v| v.as_u64())
        {
            return t.clamp(5, 600);
        }

        // 2. Base timeout by evaluation type
        let base_secs: u64 = match self.eval_type {
            EvaluationType::ExactMatch
            | EvaluationType::Regex
            | EvaluationType::Numeric
            | EvaluationType::MultipleChoice
            | EvaluationType::ToolCall => 150,
            EvaluationType::JsonSchema | EvaluationType::CodeExecution => 180,
            EvaluationType::LlmJudge => 180,
            EvaluationType::AgentTrajectory => {
                let turns = self.max_turns.unwrap_or(8).clamp(1, 64) as u64;
                (90 + turns * 45).clamp(180, 480)
            }
        };

        // 3. Difficulty multiplier
        let diff_mult: f64 = if let Some(diff) = self
            .difficulty
            .as_deref()
            .or_else(|| self.metadata.get("difficulty").and_then(|v| v.as_str()))
        {
            match diff.to_lowercase().as_str() {
                "l1" | "easy" | "simple" => 0.8,
                "l2" | "medium" => 1.0,
                "l3" | "hard" => 1.3,
                "l4" | "complex_agent" | "expert" => 1.6,
                "l5" | "olympiad" | "frontier" => 2.0,
                _ => 1.0,
            }
        } else if self.tags.iter().any(|t| {
            matches!(
                t.as_str(),
                "olympiad" | "deep_reasoning" | "complex_refactor" | "logic" | "puzzle" | "math"
            )
        }) {
            1.5
        } else {
            1.0
        };

        let calculated = ((base_secs as f64) * diff_mult).round() as u64;
        calculated.clamp(60, 600)
    }

    /// Resolve canonical difficulty tier (L1 to L5)
    pub fn resolved_tier(&self) -> &str {
        if let Some(ref d) = self.difficulty {
            return d.as_str();
        }
        if let Some(d) = self.metadata.get("difficulty").and_then(|v| v.as_str()) {
            return d;
        }
        if self
            .tags
            .iter()
            .any(|t| t == "simple" || t == "easy" || t == "sanity")
        {
            return "L1";
        }
        if self
            .tags
            .iter()
            .any(|t| t == "hard" || t == "complex_agent")
        {
            return "L4";
        }
        crate::metrics::resolve_tier(&self.id, self.category.as_str())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub test_cases: Vec<TestCase>,
}

impl Dataset {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            version: Some("1.0.0".to_string()),
            test_cases: Vec::new(),
        }
    }

    /// Filter by a top-level dataset group or an exact category/subcategory.
    ///
    /// The `category` field is a subcategory (e.g. `math_logic`, `swe`), while
    /// the product/README groups are directory-level (`foundation`, `agent`,
    /// `domain`, `safety`). The loader records the directory group in
    /// `metadata.dataset_group`; this method also supports exact subcategory
    /// aliases such as `coding`, `math`, `data`.
    pub fn filter_by_group(&self, group: &str) -> Self {
        let g = group.trim().to_lowercase();
        let matches = |case: &TestCase| -> bool {
            if g == "all" {
                return true;
            }
            let top_level = matches!(g.as_str(), "foundation" | "agent" | "domain" | "safety");
            if let Some(dataset_group) = case.metadata.get("dataset_group").and_then(|v| v.as_str())
            {
                if top_level {
                    // A directory-level group is authoritative: do not let a
                    // foundation `swe` case leak into the `agent` group merely
                    // because both use category="swe".
                    return dataset_group.eq_ignore_ascii_case(&g);
                }
                if dataset_group.eq_ignore_ascii_case(&g) {
                    return true;
                }
            }
            match g.as_str() {
                "foundation" => matches!(
                    case.category,
                    Category::Foundation
                        | Category::MathLogic
                        | Category::Instruction
                        | Category::StructuredOutput
                        | Category::LongContext
                        | Category::Multilingual
                ),
                "agent" => matches!(
                    case.category,
                    Category::Agent
                        | Category::Swe
                        | Category::Devops
                        | Category::Security
                        | Category::DataAnalyst
                ),
                "domain" => matches!(
                    case.category,
                    Category::Medical
                        | Category::Legal
                        | Category::Finance
                        | Category::Science
                        | Category::Humanities
                ),
                "safety" => matches!(case.category, Category::Safety),
                "coding" | "swe" | "swe_bench" => matches!(case.category, Category::Swe),
                "math" | "math_logic" => matches!(case.category, Category::MathLogic),
                "data" | "data_analyst" | "sql" => matches!(case.category, Category::DataAnalyst),
                "security" | "cybersecurity" => matches!(case.category, Category::Security),
                "devops" | "sysadmin" => matches!(case.category, Category::Devops),
                "medical" | "health" => matches!(case.category, Category::Medical),
                "legal" | "law" => matches!(case.category, Category::Legal),
                "finance" | "econ" => matches!(case.category, Category::Finance),
                "science" | "natsci" => matches!(case.category, Category::Science),
                "humanities" | "philosophy" => matches!(case.category, Category::Humanities),
                "instruction" => matches!(case.category, Category::Instruction),
                "structured" | "structured_output" => {
                    matches!(case.category, Category::StructuredOutput)
                }
                "long_context" => matches!(case.category, Category::LongContext),
                "multilingual" => matches!(case.category, Category::Multilingual),
                other => case.category.as_str() == other,
            }
        };
        Self {
            name: format!("{}-group-{}", self.name, group),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| matches(c))
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_category(&self, cat: &Category) -> Self {
        Self {
            name: format!("{}-{}", self.name, cat.as_str()),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| &c.category == cat)
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_tag(&self, tag: &str) -> Self {
        Self {
            name: format!("{}-tag-{}", self.name, tag),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| c.tags.iter().any(|t| t == tag))
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_difficulty(&self, diff_pattern: &str) -> Self {
        let targets: Vec<String> = diff_pattern
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            name: format!("{}-diff-{}", self.name, diff_pattern),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| {
                    let d = c.resolved_tier().to_lowercase();
                    targets.iter().any(|t| {
                        if t == "frontier" {
                            d == "l4" || d == "l5" || d == "expert" || d == "olympiad"
                        } else {
                            d == *t || c.tags.iter().any(|tag| tag.to_lowercase() == *t)
                        }
                    })
                })
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_eval_type(&self, eval_type: &EvaluationType) -> Self {
        Self {
            name: format!("{}-{:?}", self.name, eval_type),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| &c.eval_type == eval_type)
                .cloned()
                .collect(),
        }
    }

    pub fn filter_by_ids(&self, ids: &std::collections::HashSet<String>) -> Self {
        Self {
            name: format!("{}-filtered-ids", self.name),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self
                .test_cases
                .iter()
                .filter(|c| ids.contains(&c.id))
                .cloned()
                .collect(),
        }
    }

    pub fn limit(&self, max_cases: usize) -> Self {
        Self {
            name: format!("{}-limit-{}", self.name, max_cases),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: self.test_cases.iter().take(max_cases).cloned().collect(),
        }
    }

    pub fn sample(&self, ratio: f64, seed: Option<u64>) -> Self {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        let ratio = ratio.clamp(0.0, 1.0);
        let sample_count = ((self.test_cases.len() as f64) * ratio).ceil() as usize;

        let mut cases = self.test_cases.clone();
        if let Some(s) = seed {
            let mut rng = rand::rngs::StdRng::seed_from_u64(s);
            cases.shuffle(&mut rng);
        } else {
            let mut rng = rand::thread_rng();
            cases.shuffle(&mut rng);
        }
        cases.truncate(sample_count);

        Self {
            name: format!("{}-sample-{:.2}", self.name, ratio),
            description: self.description.clone(),
            version: self.version.clone(),
            test_cases: cases,
        }
    }
}

#[cfg(test)]
mod group_filter_tests {
    use super::*;
    use crate::dataset::DatasetLoader;
    use std::path::PathBuf;

    fn load_all() -> Dataset {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../datasets");
        let mut dataset = Dataset::new("all");
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    dataset
                        .test_cases
                        .extend(DatasetLoader::load_from_jsonl(&path).unwrap().test_cases);
                }
            }
        }
        dataset
    }

    #[test]
    fn group_counts_match_readme() {
        let all = load_all();
        assert_eq!(all.test_cases.len(), 1414);
        assert_eq!(all.filter_by_group("foundation").test_cases.len(), 964);
        assert_eq!(all.filter_by_group("agent").test_cases.len(), 100);
        assert_eq!(all.filter_by_group("domain").test_cases.len(), 150);
        assert_eq!(all.filter_by_group("safety").test_cases.len(), 200);
        assert_eq!(all.filter_by_group("all").test_cases.len(), 1414);
        assert!(!all.filter_by_group("foundation").test_cases.is_empty());
    }
}
