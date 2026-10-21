use anyhow::{bail, Result};
use eval_core::dataset::{DatasetLoader, EvaluationType};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn collect_jsonl_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            if child_path.is_dir() {
                collect_jsonl_files(&child_path, files)?;
            } else if child_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                files.push(child_path);
            }
        }
    }
    Ok(())
}

pub fn execute_validate(dataset_paths: Vec<String>) -> Result<()> {
    if dataset_paths.is_empty() {
        bail!("Please provide at least one dataset path to validate.");
    }

    let mut all_files = Vec::new();
    for path_str in &dataset_paths {
        let p = Path::new(path_str);
        if !p.exists() {
            eprintln!("❌ File not found: {path_str}");
            continue;
        }
        collect_jsonl_files(p, &mut all_files)?;
    }

    if all_files.is_empty() {
        bail!("No .jsonl files found in specified paths.");
    }
    all_files.sort();

    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut total_cases = 0usize;
    let mut total_errors = 0usize;

    for p in &all_files {
        println!("🔍 Validating dataset: {}", p.display());
        match DatasetLoader::load_from_jsonl(p) {
            Ok(dataset) => {
                let case_count = dataset.test_cases.len();
                total_cases += case_count;
                println!("  Loaded {case_count} test cases.");
                let mut file_errors = 0usize;

                for (idx, tc) in dataset.test_cases.iter().enumerate() {
                    let mut errors: Vec<String> = Vec::new();
                    let where_ = format!("{} #{} ({})", p.display(), idx + 1, tc.id);

                    if tc.id.trim().is_empty() {
                        errors.push("id is empty".to_string());
                    }
                    if !seen_ids.insert(tc.id.clone()) {
                        errors.push("duplicate test case id (global)".to_string());
                    }
                    if tc.prompt.trim().is_empty() {
                        errors.push("prompt is empty".to_string());
                    }
                    if let Some(difficulty) = &tc.difficulty {
                        if !matches!(difficulty.as_str(), "L1" | "L2" | "L3" | "L4" | "L5") {
                            errors.push(format!(
                                "invalid difficulty '{difficulty}' (expected L1-L5)"
                            ));
                        }
                    }

                    match tc.eval_type {
                        EvaluationType::ExactMatch => match &tc.reference_answer {
                            Some(a) if !a.trim().is_empty() => {}
                            _ => errors.push(
                                "exact_match requires a non-empty reference_answer".to_string(),
                            ),
                        },
                        EvaluationType::Regex => {
                            let pattern = tc.criteria.as_deref().or(tc.reference_answer.as_deref());
                            match pattern {
                                Some(p) if !p.trim().is_empty() => {
                                    if let Err(e) = Regex::new(p) {
                                        errors.push(format!("invalid regex: {e}"));
                                    }
                                }
                                _ => errors.push(
                                    "regex requires criteria (or reference_answer as pattern)"
                                        .to_string(),
                                ),
                            }
                            if let Some(value) = tc.metadata.get("must_not_match") {
                                let patterns: Vec<&str> = match value {
                                    serde_json::Value::String(s) => vec![s.as_str()],
                                    serde_json::Value::Array(a) => {
                                        a.iter().filter_map(|v| v.as_str()).collect()
                                    }
                                    _ => Vec::new(),
                                };
                                for forbidden in patterns {
                                    if let Err(e) = Regex::new(forbidden) {
                                        errors.push(format!(
                                            "invalid must_not_match regex '{forbidden}': {e}"
                                        ));
                                    }
                                }
                            }
                        }
                        EvaluationType::JsonSchema => match &tc.schema {
                            Some(schema) => {
                                if let Err(e) = jsonschema::JSONSchema::compile(schema) {
                                    errors.push(format!("invalid JSON schema: {e}"));
                                }
                            }
                            None => errors.push("json_schema requires a schema".to_string()),
                        },
                        EvaluationType::CodeExecution => match &tc.test_code {
                            Some(t) if !t.trim().is_empty() => {}
                            _ => errors
                                .push("code_execution requires a non-empty test_code".to_string()),
                        },
                        EvaluationType::Numeric => match &tc.reference_answer {
                            Some(a) if a.trim().replace(',', "").parse::<f64>().is_ok() => {}
                            _ => errors
                                .push("numeric requires a numeric reference_answer".to_string()),
                        },
                        EvaluationType::MultipleChoice => match &tc.reference_answer {
                            Some(a)
                                if a.trim().len() == 1
                                    && a.trim()
                                        .chars()
                                        .next()
                                        .map(|c| c.is_ascii_alphabetic())
                                        .unwrap_or(false) => {}
                            _ => errors.push(
                                "multiple_choice requires a single letter reference_answer"
                                    .to_string(),
                            ),
                        },
                        EvaluationType::F1 => match &tc.reference_answer {
                            Some(a) if !a.trim().is_empty() => {}
                            _ => {
                                errors.push("f1 requires a non-empty reference_answer".to_string())
                            }
                        },
                        EvaluationType::IfEval => {
                            if tc
                                .metadata
                                .get("instruction_id_list")
                                .and_then(|v| v.as_array())
                                .map(|a| a.is_empty())
                                .unwrap_or(true)
                            {
                                errors.push(
                                    "ifeval requires metadata.instruction_id_list".to_string(),
                                );
                            }
                            if tc
                                .metadata
                                .get("kwargs")
                                .and_then(|v| v.as_array())
                                .map(|a| a.len())
                                != tc
                                    .metadata
                                    .get("instruction_id_list")
                                    .and_then(|v| v.as_array())
                                    .map(|a| a.len())
                            {
                                errors.push(
                                    "ifeval metadata.kwargs length must match instruction_id_list"
                                        .to_string(),
                                );
                            }
                        }
                        EvaluationType::ToolCall => {
                            if tc.tools.as_deref().map(|t| t.is_empty()).unwrap_or(true) {
                                errors.push("tool_call requires non-empty tools".to_string());
                            }
                            match &tc.reference_answer {
                                Some(r) => match serde_json::from_str::<serde_json::Value>(r) {
                                    Ok(serde_json::Value::Array(_)) => {}
                                    Ok(_) => errors.push(
                                        "tool_call reference_answer must be a JSON array"
                                            .to_string(),
                                    ),
                                    Err(e) => errors.push(format!(
                                        "tool_call reference_answer is invalid JSON: {e}"
                                    )),
                                },
                                None => {
                                    errors.push("tool_call requires reference_answer".to_string())
                                }
                            }
                        }
                        EvaluationType::LlmJudge => {
                            if tc
                                .criteria
                                .as_deref()
                                .map(|s| s.trim().is_empty())
                                .unwrap_or(true)
                                && tc
                                    .reference_answer
                                    .as_deref()
                                    .map(|s| s.trim().is_empty())
                                    .unwrap_or(true)
                            {
                                errors.push(
                                    "llm_judge requires criteria or reference_answer".to_string(),
                                );
                            }
                        }
                        EvaluationType::AgentTrajectory => {
                            let has_verify = tc
                                .test_code
                                .as_deref()
                                .map(|s| !s.trim().is_empty())
                                .unwrap_or(false);
                            let has_ref = tc
                                .reference_answer
                                .as_deref()
                                .map(|s| !s.trim().is_empty())
                                .unwrap_or(false);
                            let has_criteria = tc
                                .criteria
                                .as_deref()
                                .map(|s| !s.trim().is_empty())
                                .unwrap_or(false);
                            if !has_verify && !has_ref && !has_criteria {
                                errors.push("agent_trajectory needs test_code, reference_answer, or criteria".to_string());
                            }
                        }
                    }

                    for error in errors {
                        eprintln!("  ❌ {where_}: {error}");
                        file_errors += 1;
                    }
                }

                total_errors += file_errors;
                if file_errors == 0 {
                    println!("  ✅ All test cases passed schema validation.");
                } else {
                    eprintln!("  ❌ {file_errors} error(s) in this file.");
                }
            }
            Err(e) => {
                total_errors += 1;
                eprintln!("  ❌ Dataset parse error: {e}");
            }
        }
    }

    println!(
        "\n📊 Validation Summary: {} files checked, {} total test cases, {} errors.",
        all_files.len(),
        total_cases,
        total_errors
    );
    if total_errors > 0 {
        bail!("Dataset validation failed with {total_errors} error(s)");
    }
    Ok(())
}
