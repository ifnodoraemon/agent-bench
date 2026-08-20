use anyhow::Result;
use eval_core::dataset::{DatasetLoader, EvaluationType};
use std::fs;
use std::path::{Path, PathBuf};

fn collect_jsonl_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
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
        println!("⚠️ Please provide at least one dataset path to validate.");
        return Ok(());
    }

    let mut all_files = Vec::new();
    for path_str in &dataset_paths {
        let p = Path::new(path_str);
        if !p.exists() {
            println!("❌ File not found: {path_str}");
            continue;
        }
        collect_jsonl_files(p, &mut all_files)?;
    }

    if all_files.is_empty() {
        println!("⚠️ No .jsonl files found in specified paths.");
        return Ok(());
    }

    all_files.sort();

    let mut total_cases = 0;
    let mut total_errors = 0;

    for p in &all_files {
        println!("🔍 Validating dataset: {}", p.display());
        match DatasetLoader::load_from_jsonl(p) {
            Ok(dataset) => {
                let case_count = dataset.test_cases.len();
                total_cases += case_count;
                println!("  ✅ Successfully loaded {} test cases.", case_count);
                let mut errors = 0;
                for (idx, tc) in dataset.test_cases.iter().enumerate() {
                    match tc.eval_type {
                        EvaluationType::ExactMatch => {
                            if tc.reference_answer.is_none() {
                                println!("  ⚠️ Case #{} ({}): ExactMatch requires reference_answer", idx + 1, tc.id);
                                errors += 1;
                            }
                        }
                        EvaluationType::JsonSchema => {
                            if tc.schema.is_none() {
                                println!("  ⚠️ Case #{} ({}): JsonSchema requires schema definition", idx + 1, tc.id);
                                errors += 1;
                            }
                        }
                        EvaluationType::CodeExecution => {
                            if tc.test_code.is_none() {
                                println!("  ⚠️ Case #{} ({}): CodeExecution requires test_code", idx + 1, tc.id);
                                errors += 1;
                            }
                        }
                        _ => {}
                    }
                }
                total_errors += errors;
                if errors == 0 {
                    println!("  ✨ All test cases passed dataset schema validation!");
                } else {
                    println!("  ⚠️ Found {errors} warning(s) in dataset.");
                }
            }
            Err(e) => {
                total_errors += 1;
                println!("  ❌ Dataset parse error: {e}");
            }
        }
    }

    println!("\n📊 Validation Summary: {} files checked, {} total test cases, {} errors/warnings.", all_files.len(), total_cases, total_errors);
    Ok(())
}
