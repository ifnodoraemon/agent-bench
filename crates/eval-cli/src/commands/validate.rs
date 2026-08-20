use anyhow::Result;
use eval_core::dataset::{DatasetLoader, EvaluationType};
use std::path::Path;

pub fn execute_validate(dataset_paths: Vec<String>) -> Result<()> {
    if dataset_paths.is_empty() {
        println!("⚠️ Please provide at least one dataset path to validate.");
        return Ok(());
    }

    for path_str in dataset_paths {
        let p = Path::new(&path_str);
        if !p.exists() {
            println!("❌ File not found: {path_str}");
            continue;
        }

        println!("🔍 Validating dataset: {}", p.display());
        match DatasetLoader::load_from_jsonl(p) {
            Ok(dataset) => {
                println!("  ✅ Successfully loaded {} test cases.", dataset.test_cases.len());
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
                if errors == 0 {
                    println!("  ✨ All test cases passed dataset schema validation!");
                } else {
                    println!("  ⚠️ Found {errors} warning(s) in dataset.");
                }
            }
            Err(e) => {
                println!("  ❌ Dataset parse error: {e}");
            }
        }
    }

    Ok(())
}
