use anyhow::{anyhow, Result};
use eval_core::model::ToolDefinition;
use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Return the 5 primitive Pi/SWE-bench coding tools
pub fn get_pi_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::function(
            "bash",
            "Execute a bash shell command in the workspace directory. Use this to run tests, inspect logs, compile, or execute scripts.",
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line string to execute, e.g. 'pytest tests/' or 'cargo test' or 'python3 main.py'"
                    }
                },
                "required": ["command"]
            }),
        ),
        ToolDefinition::function(
            "read_file",
            "Read file content from the workspace. Supports specifying offset and line count limit for large files.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Optional 1-indexed starting line number (defaults to 1)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional maximum number of lines to read (defaults to 100)"
                    }
                },
                "required": ["path"]
            }),
        ),
        ToolDefinition::function(
            "write_file",
            "Create a new file or completely overwrite an existing file in the workspace.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The full text content to write into the file"
                    }
                },
                "required": ["path", "content"]
            }),
        ),
        ToolDefinition::function(
            "edit_file",
            "Perform a precise search-and-replace modification on an existing workspace file.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the file to edit"
                    },
                    "old_str": {
                        "type": "string",
                        "description": "Exact text chunk in the file to be replaced (must be unique)"
                    },
                    "new_str": {
                        "type": "string",
                        "description": "New replacement text"
                    }
                },
                "required": ["path", "old_str", "new_str"]
            }),
        ),
        ToolDefinition::function(
            "grep_search",
            "Search for a regex or text query across files in the workspace directory.",
            json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search term or regular expression"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory or file to restrict search (defaults to '.')"
                    }
                },
                "required": ["query"]
            }),
        ),
    ]
}

/// Execute a Pi-style primitive tool in the workspace directory
pub fn execute_pi_tool(workspace_root: &Path, name: &str, arguments_json: &str) -> Result<String> {
    let parsed_args: Value = serde_json::from_str(arguments_json)
        .map_err(|e| anyhow!("Invalid JSON arguments for tool '{name}': {e}"))?;

    match name {
        "bash" | "run_command" => {
            let cmd = parsed_args
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("");

            if cmd.trim().is_empty() {
                return Ok("Error: No command provided.".to_string());
            }

            let output = Command::new("bash")
                .arg("-c")
                .arg(cmd)
                .current_dir(workspace_root)
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(stdout.trim_end());
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&format!("[stderr]: {}", stderr.trim_end()));
            }
            if exit_code != 0 && result.is_empty() {
                result.push_str(&format!("[Exit code: {exit_code}]"));
            }
            if result.is_empty() {
                result.push_str("(Command completed with no output, exit code 0)");
            }

            Ok(result)
        }
        "read_file" | "view_file" => {
            let rel_path = parsed_args
                .get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| anyhow!("Missing 'path' parameter in read_file"))?;

            let full_path = sanitize_path(workspace_root, rel_path)?;
            if !full_path.exists() {
                return Ok(format!("Error: File not found: {rel_path}"));
            }

            let content = fs::read_to_string(&full_path)?;
            let offset = parsed_args
                .get("offset")
                .and_then(|o| o.as_u64())
                .unwrap_or(1) as usize;
            let limit = parsed_args
                .get("limit")
                .and_then(|l| l.as_u64())
                .unwrap_or(100) as usize;

            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();

            let start_idx = offset.saturating_sub(1);
            let end_idx = (start_idx + limit).min(total_lines);

            if start_idx >= total_lines {
                return Ok(format!(
                    "File has only {} lines. Requested offset {} is out of range.",
                    total_lines, offset
                ));
            }

            let mut output = format!("File: {rel_path} (Lines {}-{} of {})\n", start_idx + 1, end_idx, total_lines);
            for (idx, line) in lines[start_idx..end_idx].iter().enumerate() {
                output.push_str(&format!("{:4} | {}\n", start_idx + idx + 1, line));
            }

            Ok(output)
        }
        "write_file" => {
            let rel_path = parsed_args
                .get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| anyhow!("Missing 'path' parameter in write_file"))?;
            let content = parsed_args
                .get("content")
                .and_then(|c| c.as_str())
                .ok_or_else(|| anyhow!("Missing 'content' parameter in write_file"))?;

            let full_path = sanitize_path(workspace_root, rel_path)?;
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&full_path, content)?;
            Ok(format!("Successfully written {} bytes to {}", content.len(), rel_path))
        }
        "edit_file" => {
            let rel_path = parsed_args
                .get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| anyhow!("Missing 'path' parameter in edit_file"))?;
            let old_str = parsed_args
                .get("old_str")
                .and_then(|o| o.as_str())
                .ok_or_else(|| anyhow!("Missing 'old_str' parameter in edit_file"))?;
            let new_str = parsed_args
                .get("new_str")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow!("Missing 'new_str' parameter in edit_file"))?;

            let full_path = sanitize_path(workspace_root, rel_path)?;
            if !full_path.exists() {
                return Ok(format!("Error: File not found: {rel_path}"));
            }

            let content = fs::read_to_string(&full_path)?;
            let occurrences = content.matches(old_str).count();

            if occurrences == 0 {
                return Ok(format!(
                    "Error: 'old_str' not found in {rel_path}. Make sure whitespace and formatting match exactly."
                ));
            } else if occurrences > 1 {
                return Ok(format!(
                    "Error: 'old_str' matched {occurrences} times in {rel_path}. Please provide more unique context."
                ));
            }

            let new_content = content.replacen(old_str, new_str, 1);
            fs::write(&full_path, new_content)?;
            Ok(format!("Successfully replaced 1 occurrence in {rel_path}."))
        }
        "grep_search" => {
            let query = parsed_args
                .get("query")
                .and_then(|q| q.as_str())
                .ok_or_else(|| anyhow!("Missing 'query' parameter in grep_search"))?;
            let target_subpath = parsed_args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or(".");

            let search_dir = sanitize_path(workspace_root, target_subpath)?;
            let re = match Regex::new(query) {
                Ok(r) => r,
                Err(_) => {
                    Regex::new(&regex::escape(query))?
                }
            };

            let mut matches = Vec::new();
            search_recursive(workspace_root, &search_dir, &re, &mut matches, 50)?;

            if matches.is_empty() {
                Ok(format!("No matches found for pattern '{query}'."))
            } else {
                Ok(matches.join("\n"))
            }
        }
        other => Err(anyhow!("Unknown Pi tool: {other}")),
    }
}

pub fn sanitize_path(workspace_root: &Path, rel_path: &str) -> Result<PathBuf> {
    let clean = rel_path.trim_start_matches('/').trim_start_matches("./");
    let full = workspace_root.join(clean);
    
    // Normalization check to prevent directory traversal out of workspace root
    let root_canon = workspace_root.canonicalize().unwrap_or_else(|_| workspace_root.to_path_buf());
    let target_canon = if full.exists() {
        full.canonicalize().unwrap_or(full.clone())
    } else {
        full.clone()
    };

    if target_canon.starts_with(&root_canon) || full.starts_with(workspace_root) {
        Ok(full)
    } else {
        Err(anyhow!("Security Error: Path traversal outside workspace denied: {rel_path}"))
    }
}

fn search_recursive(
    workspace_root: &Path,
    current_dir: &Path,
    re: &Regex,
    results: &mut Vec<String>,
    max_results: usize,
) -> Result<()> {
    if !current_dir.exists() {
        return Ok(());
    }

    if current_dir.is_file() {
        search_file(workspace_root, current_dir, re, results, max_results)?;
        return Ok(());
    }

    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if dir_name == ".git" || dir_name == "target" || dir_name == "node_modules" {
                continue;
            }
            search_recursive(workspace_root, &path, re, results, max_results)?;
        } else {
            search_file(workspace_root, &path, re, results, max_results)?;
        }
        if results.len() >= max_results {
            break;
        }
    }

    Ok(())
}

fn search_file(
    workspace_root: &Path,
    file_path: &Path,
    re: &Regex,
    results: &mut Vec<String>,
    max_results: usize,
) -> Result<()> {
    let Ok(content) = fs::read_to_string(file_path) else {
        return Ok(());
    };

    let rel = file_path
        .strip_prefix(workspace_root)
        .unwrap_or(file_path)
        .display()
        .to_string();

    for (line_idx, line) in content.lines().enumerate() {
        if re.is_match(line) {
            results.push(format!("{}:{}: {}", rel, line_idx + 1, line.trim()));
            if results.len() >= max_results {
                results.push(format!("... (Capped at {max_results} matches)"));
                break;
            }
        }
    }

    Ok(())
}
