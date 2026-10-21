use crate::calc;
use crate::path;
use anyhow::{anyhow, Context, Result};
use eval_core::model::ToolDefinition;
use eval_core::sandbox::Sandbox;
use regex::Regex;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_GREP_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1024;

/// Return standard Pi/SWE-bench coding and agent tools.
pub fn get_pi_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::function(
            "bash",
            "Execute a bash shell command in the isolated workspace. The command runs with no network access, a cleared environment and a 60s limit.",
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line string to execute, e.g. 'pytest tests/' or 'python3 main.py'"
                    }
                },
                "required": ["command"]
            }),
        ),
        ToolDefinition::function(
            "read_file",
            "Read file content from the workspace. Supports offset and line count limit for large files.",
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative path of the file to read"},
                    "offset": {"type": "integer", "description": "Optional 1-indexed starting line number (defaults to 1)"},
                    "limit": {"type": "integer", "description": "Optional maximum number of lines to read (defaults to 100)"}
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
                    "path": {"type": "string", "description": "Relative path of the file to write"},
                    "content": {"type": "string", "description": "The full text content to write into the file"}
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
                    "path": {"type": "string", "description": "Relative path of the file to edit"},
                    "old_str": {"type": "string", "description": "Exact text chunk in the file to be replaced (must be unique)"},
                    "new_str": {"type": "string", "description": "New replacement text"}
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
                    "query": {"type": "string", "description": "Search term or regular expression"},
                    "path": {"type": "string", "description": "Optional subdirectory or file to restrict search (defaults to '.')"}
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::function(
            "calculator",
            "Evaluate an arithmetic expression safely (numbers, + - * / % ^, parentheses, and a fixed math function allowlist).",
            json!({
                "type": "object",
                "properties": {
                    "expression": {"type": "string", "description": "Mathematical expression, e.g. '(5000 + 250) / 3' or 'sqrt(144)'"}
                },
                "required": ["expression"]
            }),
        ),
        ToolDefinition::function(
            "search_web",
            "Search via an operator-configured SearXNG-compatible JSON endpoint (AGENT_BENCH_SEARCH_ENDPOINT). No built-in fake results; returns a clear error when unconfigured.",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query string"}
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::function(
            "database_query",
            "Execute a read-only SELECT/WITH query against a real SQLite database inside the workspace (default data/bench.db).",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "SQL-like or natural-language query string"}
                },
                "required": ["query"]
            }),
        ),
    ]
}

/// Compatibility helper used by tests and older callers. Each call creates a
/// fresh Sandbox configured from the environment.
pub fn execute_pi_tool(workspace_root: &Path, name: &str, arguments_json: &str) -> Result<String> {
    let sandbox = Sandbox::from_env();
    execute_pi_tool_with_sandbox(workspace_root, name, arguments_json, &sandbox)
}

pub fn execute_pi_tool_with_sandbox(
    workspace_root: &Path,
    name: &str,
    arguments_json: &str,
    sandbox: &Sandbox,
) -> Result<String> {
    let parsed_args: Value = serde_json::from_str(arguments_json)
        .map_err(|e| anyhow!("Invalid JSON arguments for tool '{name}': {e}"))?;

    match name {
        "bash" | "run_command" | "sh" | "terminal" => {
            let cmd = parsed_args
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            if cmd.trim().is_empty() {
                return Ok("Error: No command provided.".to_string());
            }

            let effective_cmd = path::rewrite_app_alias(cmd, workspace_root);
            let out = sandbox.run_bash(&effective_cmd, workspace_root, Duration::from_secs(60))?;
            Ok(format_exec_output(out))
        }
        "read_file" | "view_file" => {
            let rel_path = parsed_args
                .get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| anyhow!("Missing 'path' parameter in read_file"))?;

            let full_path = path::safe_join(workspace_root, rel_path)?;
            let meta = fs::symlink_metadata(&full_path)
                .map_err(|_| anyhow!("File not found: {rel_path}"))?;
            if meta.file_type().is_symlink() {
                return Err(anyhow!("Refusing to read symlink: {rel_path}"));
            }
            if !meta.is_file() {
                return Err(anyhow!("Not a regular file: {rel_path}"));
            }
            if meta.len() > MAX_FILE_BYTES {
                return Err(anyhow!(
                    "File is too large to read ({} bytes; max {MAX_FILE_BYTES})",
                    meta.len()
                ));
            }

            let content = path::read_string_no_follow(workspace_root, rel_path)?;
            let offset = parsed_args
                .get("offset")
                .and_then(|o| o.as_u64())
                .unwrap_or(1) as usize;
            let limit = parsed_args
                .get("limit")
                .and_then(|l| l.as_u64())
                .unwrap_or(100)
                .min(10_000) as usize;

            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();
            let start_idx = offset.saturating_sub(1);
            let end_idx = start_idx.saturating_add(limit).min(total_lines);

            if start_idx >= total_lines {
                return Ok(format!(
                    "File has only {} lines. Requested offset {} is out of range.",
                    total_lines, offset
                ));
            }

            let mut output = format!(
                "File: {rel_path} (Lines {}-{} of {})\n",
                start_idx + 1,
                end_idx,
                total_lines
            );
            for (idx, line) in lines[start_idx..end_idx].iter().enumerate() {
                output.push_str(&format!("{:4} | {}\n", start_idx + idx + 1, line));
            }
            Ok(cap_string(output))
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
            if content.len() as u64 > MAX_FILE_BYTES {
                return Err(anyhow!(
                    "Content is too large ({} bytes; max {MAX_FILE_BYTES})",
                    content.len()
                ));
            }
            path::write_no_follow(workspace_root, rel_path, content.as_bytes())?;
            Ok(format!(
                "Successfully written {} bytes to {}",
                content.len(),
                rel_path
            ))
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
            if old_str.is_empty() {
                return Err(anyhow!("old_str must not be empty"));
            }

            let full_path = path::safe_join(workspace_root, rel_path)?;
            let meta = fs::symlink_metadata(&full_path)
                .map_err(|_| anyhow!("File not found: {rel_path}"))?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(anyhow!("Refusing to edit non-regular file: {rel_path}"));
            }
            if meta.len() > MAX_FILE_BYTES {
                return Err(anyhow!("File is too large to edit"));
            }

            let content = path::read_string_no_follow(workspace_root, rel_path)?;
            let occurrences = content.matches(old_str).count();
            if occurrences == 0 {
                return Ok(format!(
                    "Error: 'old_str' not found in {rel_path}. Make sure whitespace and formatting match exactly."
                ));
            }
            if occurrences > 1 {
                return Ok(format!(
                    "Error: 'old_str' matched {occurrences} times in {rel_path}. Please provide more unique context."
                ));
            }
            let new_content = content.replacen(old_str, new_str, 1);
            if new_content.len() as u64 > MAX_FILE_BYTES {
                return Err(anyhow!("Edited file would exceed maximum size"));
            }
            path::write_no_follow(workspace_root, rel_path, new_content.as_bytes())?;
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

            let search_dir = if target_subpath.is_empty() || target_subpath == "." {
                workspace_root.to_path_buf()
            } else {
                path::safe_join(workspace_root, target_subpath)?
            };

            let re = match Regex::new(query) {
                Ok(r) => r,
                Err(_) => Regex::new(&regex::escape(query))?,
            };

            let mut matches = Vec::new();
            let mut visited = 0usize;
            search_recursive(
                workspace_root,
                &search_dir,
                &re,
                &mut matches,
                0,
                &mut visited,
            )?;

            if matches.is_empty() {
                Ok(format!("No matches found for pattern '{query}'."))
            } else {
                Ok(cap_string(matches.join("\n")))
            }
        }
        "calculator" | "calculate" | "calc" => {
            let expr = parsed_args
                .get("expression")
                .or_else(|| parsed_args.get("expr"))
                .or_else(|| parsed_args.get("query"))
                .and_then(|e| e.as_str())
                .unwrap_or("0");
            Ok(calc::evaluate_to_string(expr)?)
        }
        "search_web" | "web_search" | "search" => {
            let query = parsed_args
                .get("query")
                .or_else(|| parsed_args.get("q"))
                .and_then(|q| q.as_str())
                .unwrap_or("");
            search_web_tool(query)
        }
        "database_query" | "query_database" | "sql_query" | "db_query" => {
            database_query_tool(workspace_root, &parsed_args)
        }
        other => Err(anyhow!("Unknown Pi tool: {other}")),
    }
}

/// Real web search through an operator-configured SearXNG-compatible JSON
/// endpoint. The endpoint is trusted configuration, never model-provided.
pub fn search_web_tool(query: &str) -> Result<String> {
    let query = query.trim();
    if query.is_empty() {
        return Err(anyhow!("search_web requires a non-empty query"));
    }
    let endpoint = std::env::var("AGENT_BENCH_SEARCH_ENDPOINT").map_err(|_| {
        anyhow!(
            "search_web is not configured. Set AGENT_BENCH_SEARCH_ENDPOINT to a \
             SearXNG-compatible JSON endpoint (e.g. http://127.0.0.1:8080/search)."
        )
    })?;
    let endpoint = endpoint.trim();
    if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
        return Err(anyhow!(
            "AGENT_BENCH_SEARCH_ENDPOINT must be an http(s) URL"
        ));
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build search HTTP client")?;
    let mut req = client
        .get(endpoint)
        .query(&[("q", query), ("format", "json")]);
    if let Ok(key) = std::env::var("AGENT_BENCH_SEARCH_API_KEY") {
        if !key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key.trim()));
        }
    }
    let resp = req.send().context("search request failed")?;
    let status = resp.status();
    let body = resp.text().context("failed to read search response")?;
    if !status.is_success() {
        return Err(anyhow!(
            "search endpoint returned HTTP {}: {}",
            status,
            body.chars().take(300).collect::<String>()
        ));
    }
    let json: Value = serde_json::from_str(&body).context("search endpoint did not return JSON")?;
    let results = json
        .get("results")
        .or_else(|| json.pointer("/web/results"))
        .and_then(Value::as_array);
    let Some(results) = results else {
        return Err(anyhow!(
            "search endpoint JSON has no results[] or web.results[] array"
        ));
    };
    let mut lines = Vec::new();
    for item in results.iter().take(5) {
        let title = item
            .get("title")
            .or_else(|| item.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("(untitled)");
        let url = item
            .get("url")
            .or_else(|| item.get("link"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let snippet = item
            .get("content")
            .or_else(|| item.get("snippet"))
            .or_else(|| item.get("description"))
            .and_then(Value::as_str)
            .unwrap_or("");
        lines.push(format!("{title}\n{url}\n{snippet}").trim().to_string());
    }
    if lines.is_empty() {
        return Ok(format!("No live results returned for '{query}'."));
    }
    Ok(cap_string(lines.join("\n\n")))
}

/// Read-only SQLite query tool. It never executes writes, ATTACH, PRAGMA, or
/// multiple statements; the database file must resolve inside the workspace.
pub fn database_query_tool(workspace_root: &Path, args: &Value) -> Result<String> {
    let sql = args
        .get("query")
        .or_else(|| args.get("sql"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if sql.is_empty() {
        return Err(anyhow!("database_query requires a non-empty SQL query"));
    }
    let lower = sql.to_lowercase();
    if !(lower.starts_with("select") || lower.starts_with("with")) {
        return Err(anyhow!(
            "database_query only allows read-only SELECT/WITH statements"
        ));
    }
    for forbidden in [
        "insert ",
        "update ",
        "delete ",
        "drop ",
        "alter ",
        "create ",
        "attach ",
        "detach ",
        "pragma ",
        "vacuum ",
        "replace ",
        "begin ",
        "commit ",
        "rollback ",
    ] {
        if lower.contains(forbidden) {
            return Err(anyhow!(
                "database_query rejects statement containing '{forbidden}'"
            ));
        }
    }
    let db_rel = args
        .get("database")
        .and_then(Value::as_str)
        .unwrap_or("data/bench.db");
    let db_path = path::safe_join(workspace_root, db_rel)?;
    let meta = fs::symlink_metadata(&db_path)
        .map_err(|_| anyhow!("SQLite database not found: {db_rel}"))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(anyhow!("Refusing to open non-regular database: {db_rel}"));
    }
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open SQLite database: {db_rel}"))?;
    conn.busy_timeout(Duration::from_secs(5))?;
    let mut stmt = conn
        .prepare(sql)
        .with_context(|| format!("invalid SQL: {sql}"))?;
    let columns: Vec<String> = (0..stmt.column_count())
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    let mut truncated = false;
    while let Some(row) = rows.next()? {
        if out.len() >= 100 {
            truncated = true;
            break;
        }
        let mut values = Vec::with_capacity(columns.len());
        for i in 0..columns.len() {
            let value = match row.get_ref(i)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(v) => json!(v),
                ValueRef::Real(v) => json!(v),
                ValueRef::Text(t) => json!(String::from_utf8_lossy(t).to_string()),
                ValueRef::Blob(b) => json!(format!("<blob {} bytes>", b.len())),
            };
            values.push(value);
        }
        out.push(Value::Array(values));
    }
    Ok(serde_json::to_string_pretty(&json!({
        "columns": columns,
        "rows": out,
        "row_count": out.len(),
        "truncated": truncated,
    }))?)
}

fn format_exec_output(out: eval_core::sandbox::ExecOutput) -> String {
    if out.timed_out {
        return "Error: Command execution timed out (60s limit).".to_string();
    }
    let mut result = out.combined();
    if !out.success {
        let code = out
            .status_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        if !result.contains("[exit code") {
            result.push_str(&format!("\n[exit code: {code}]"));
        }
    }
    if result.is_empty() {
        result.push_str("(Command completed with no output, exit code 0)");
    }
    cap_string(result)
}

fn cap_string(s: String) -> String {
    if s.len() <= MAX_TOOL_OUTPUT_BYTES {
        return s;
    }
    let mut end = MAX_TOOL_OUTPUT_BYTES;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n[tool output truncated]", &s[..end])
}

/// Kept for compatibility with callers that used the old helper. New code
/// should call [`path::safe_join`] directly.
pub fn sanitize_path(workspace_root: &Path, rel_path: &str) -> Result<PathBuf> {
    path::safe_join(workspace_root, rel_path)
}

fn search_recursive(
    workspace_root: &Path,
    current_dir: &Path,
    re: &Regex,
    results: &mut Vec<String>,
    depth: usize,
    visited: &mut usize,
) -> Result<()> {
    if depth > 16 || results.len() >= 50 || *visited >= 10_000 {
        return Ok(());
    }
    if !current_dir.exists() {
        return Ok(());
    }

    let meta = fs::symlink_metadata(current_dir)?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_file() {
        search_file(workspace_root, current_dir, re, results)?;
        return Ok(());
    }

    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        *visited += 1;
        if *visited >= 10_000 || results.len() >= 50 {
            break;
        }
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if dir_name == ".git" || dir_name == "target" || dir_name == "node_modules" {
                continue;
            }
            search_recursive(workspace_root, &path, re, results, depth + 1, visited)?;
        } else {
            search_file(workspace_root, &path, re, results)?;
        }
        if results.len() >= 50 {
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
) -> Result<()> {
    let Ok(meta) = fs::symlink_metadata(file_path) else {
        return Ok(());
    };
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > MAX_GREP_FILE_BYTES {
        return Ok(());
    }
    let rel = match file_path.strip_prefix(workspace_root) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return Ok(()),
    };
    let Ok(content) = path::read_string_no_follow(workspace_root, &rel) else {
        return Ok(());
    };

    for (line_idx, line) in content.lines().enumerate() {
        if re.is_match(line) {
            results.push(format!("{}:{}: {}", rel, line_idx + 1, line.trim()));
            if results.len() >= 50 {
                results.push("... (Capped at 50 matches)".to_string());
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculator_rejects_code() {
        let dir = tempfile::tempdir().unwrap();
        let sb = Sandbox::from_env();
        let err = execute_pi_tool_with_sandbox(
            dir.path(),
            "calculator",
            r#"{"expression":"__import__('os').system('id')"}"#,
            &sb,
        );
        assert!(err.is_err());
    }

    #[test]
    fn write_and_read_are_workspace_relative() {
        let dir = tempfile::tempdir().unwrap();
        let sb = Sandbox::from_env();
        execute_pi_tool_with_sandbox(
            dir.path(),
            "write_file",
            r#"{"path":"notes/a.txt","content":"hello"}"#,
            &sb,
        )
        .unwrap();
        let out =
            execute_pi_tool_with_sandbox(dir.path(), "read_file", r#"{"path":"notes/a.txt"}"#, &sb)
                .unwrap();
        assert!(out.contains("hello"));
    }

    #[test]
    fn traversal_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let sb = Sandbox::from_env();
        let out = execute_pi_tool_with_sandbox(
            dir.path(),
            "write_file",
            r#"{"path":"../../escape.txt","content":"x"}"#,
            &sb,
        );
        assert!(out.is_err());
    }
}
