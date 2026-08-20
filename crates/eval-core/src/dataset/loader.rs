use super::types::{Dataset, TestCase};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct DatasetLoader;

impl DatasetLoader {
    /// Load test cases from a JSONL file (one JSON object per line)
    pub fn load_from_jsonl<P: AsRef<Path>>(path: P) -> Result<Dataset> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)
            .with_context(|| format!("Failed to open dataset file: {}", path_ref.display()))?;
        let reader = BufReader::new(file);

        let mut test_cases = Vec::new();
        for (line_idx, line) in reader.lines().enumerate() {
            let line_content = line?;
            let trimmed = line_content.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            let test_case: TestCase = serde_json::from_str(trimmed).with_context(|| {
                format!(
                    "Failed to parse JSON on line {} of {}",
                    line_idx + 1,
                    path_ref.display()
                )
            })?;
            test_cases.push(test_case);
        }

        let dataset_name = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("benchmark")
            .to_string();

        Ok(Dataset {
            name: dataset_name,
            description: None,
            version: Some("1.0.0".to_string()),
            test_cases,
        })
    }

    /// Load test cases from a single JSON file (containing an array or dataset object)
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Dataset> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)?;
        if let Ok(dataset) = serde_json::from_str::<Dataset>(&content) {
            return Ok(dataset);
        }
        let test_cases: Vec<TestCase> = serde_json::from_str(&content)?;
        let name = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("benchmark")
            .to_string();
        Ok(Dataset {
            name,
            description: None,
            version: Some("1.0.0".to_string()),
            test_cases,
        })
    }
}
