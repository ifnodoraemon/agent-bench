use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait SimulatedEnvironment: Send + Sync {
    fn execute_tool(&self, name: &str, arguments_json: &str) -> Result<String>;
    fn reset(&self);
}

#[derive(Clone)]
pub struct MockSystemEnvironment {
    files: Arc<Mutex<HashMap<String, String>>>,
    db_tables: Arc<Mutex<HashMap<String, Vec<HashMap<String, String>>>>>,
    search_knowledge: Arc<Mutex<HashMap<String, String>>>,
    error_injections: Arc<Mutex<HashMap<String, (usize, String)>>>, // tool_name -> (fail_count_remaining, error_message)
}

impl Default for MockSystemEnvironment {
    fn default() -> Self {
        let mut files = HashMap::new();
        files.insert(
            "/app/config.yaml".to_string(),
            "server:\n  port: 8080\n  host: 0.0.0.0\ndatabase:\n  url: postgres://admin:secret@localhost:5432/app_db\n".to_string(),
        );
        files.insert(
            "/app/logs/error.log".to_string(),
            "[2026-08-20 04:12:01] ERROR ConnectionRefused: database host localhost:5432 unreachable\n[2026-08-20 04:12:05] FATAL Server shutdown abnormally\n".to_string(),
        );
        files.insert(
            "/etc/os-release".to_string(),
            "NAME=\"Ubuntu\"\nVERSION=\"24.04 LTS\"\n".to_string(),
        );

        let mut db_tables = HashMap::new();
        let users = vec![
            HashMap::from([("id".into(), "1".into()), ("name".into(), "Alice".into()), ("role".into(), "admin".into()), ("balance".into(), "5000".into())]),
            HashMap::from([("id".into(), "2".into()), ("name".into(), "Bob".into()), ("role".into(), "user".into()), ("balance".into(), "250".into())]),
            HashMap::from([("id".into(), "3".into()), ("name".into(), "Charlie".into()), ("role".into(), "user".into()), ("balance".into(), "0".into())]),
        ];
        db_tables.insert("users".to_string(), users);

        let mut search_knowledge = HashMap::new();
        search_knowledge.insert(
            "tokio".to_string(),
            "Tokio is an asynchronous runtime for the Rust programming language.".to_string(),
        );
        search_knowledge.insert(
            "rust".to_string(),
            "Rust is a systems programming language that focuses on safety, speed, and concurrency.".to_string(),
        );
        search_knowledge.insert(
            "antigravity".to_string(),
            "Antigravity is an advanced agentic AI coding assistant developed by Google DeepMind.".to_string(),
        );

        Self {
            files: Arc::new(Mutex::new(files)),
            db_tables: Arc::new(Mutex::new(db_tables)),
            search_knowledge: Arc::new(Mutex::new(search_knowledge)),
            error_injections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl MockSystemEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inject_file(&self, path: impl Into<String>, content: impl Into<String>) {
        self.files.lock().unwrap().insert(path.into(), content.into());
    }

    pub fn inject_temporary_error(&self, tool_name: impl Into<String>, fail_times: usize, error_message: impl Into<String>) {
        self.error_injections.lock().unwrap().insert(tool_name.into(), (fail_times, error_message.into()));
    }
}

impl SimulatedEnvironment for MockSystemEnvironment {
    fn execute_tool(&self, name: &str, arguments_json: &str) -> Result<String> {
        // Check error injection
        {
            let mut errors = self.error_injections.lock().unwrap();
            if let Some((remaining, msg)) = errors.get_mut(name) {
                if *remaining > 0 {
                    *remaining -= 1;
                    return Ok(format!("Error: {}", msg));
                }
            }
        }

        let parsed_args: Value = serde_json::from_str(arguments_json)
            .map_err(|e| anyhow!("Invalid JSON arguments for tool '{name}': {e}"))?;

        match name {
            "bash" => {
                let cmd = parsed_args
                    .get("command")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                let files = self.files.lock().unwrap();

                if cmd.starts_with("cat ") {
                    let path = cmd.trim_start_matches("cat ").trim();
                    if let Some(content) = files.get(path) {
                        Ok(content.clone())
                    } else {
                        Ok(format!("cat: {path}: No such file or directory"))
                    }
                } else if cmd == "ls" || cmd.starts_with("ls ") {
                    let file_list: Vec<String> = files.keys().cloned().collect();
                    Ok(file_list.join("\n"))
                } else if cmd.starts_with("grep ") {
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let query = parts[1];
                        let path = parts[2];
                        if let Some(content) = files.get(path) {
                            let matches: Vec<&str> = content
                                .lines()
                                .filter(|line| line.contains(query))
                                .collect();
                            Ok(matches.join("\n"))
                        } else {
                            Ok(format!("grep: {path}: No such file or directory"))
                        }
                    } else {
                        Ok("grep: missing arguments".to_string())
                    }
                } else {
                    Ok(format!("[Mock Bash] Executed: '{cmd}' -> Exit code 0"))
                }
            }
            "search_web" => {
                let query = parsed_args
                    .get("query")
                    .and_then(|q| q.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let knowledge = self.search_knowledge.lock().unwrap();
                for (k, v) in knowledge.iter() {
                    if query.contains(k) || k.contains(&query) {
                        return Ok(format!("Search Result for '{}': {}", query, v));
                    }
                }
                Ok(format!("Search Result: No specific records found for '{query}'."))
            }
            "query_database" => {
                let sql = parsed_args
                    .get("sql")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let db = self.db_tables.lock().unwrap();
                if sql.to_lowercase().contains("users") {
                    if let Some(users) = db.get("users") {
                        return Ok(serde_json::to_string_pretty(users)?);
                    }
                }
                Ok("[] (0 rows returned)".to_string())
            }
            "calculator" => {
                let expr = parsed_args
                    .get("expression")
                    .and_then(|e| e.as_str())
                    .unwrap_or("");
                let cleaned = expr.replace(' ', "");
                if let Some(pos) = cleaned.find('+') {
                    let a: f64 = cleaned[..pos].parse().unwrap_or(0.0);
                    let b: f64 = cleaned[pos+1..].parse().unwrap_or(0.0);
                    return Ok((a + b).to_string());
                } else if let Some(pos) = cleaned.find('*') {
                    let a: f64 = cleaned[..pos].parse().unwrap_or(0.0);
                    let b: f64 = cleaned[pos+1..].parse().unwrap_or(0.0);
                    return Ok((a * b).to_string());
                } else if let Some(pos) = cleaned.find('-') {
                    let a: f64 = cleaned[..pos].parse().unwrap_or(0.0);
                    let b: f64 = cleaned[pos+1..].parse().unwrap_or(0.0);
                    return Ok((a - b).to_string());
                } else if let Some(pos) = cleaned.find('/') {
                    let a: f64 = cleaned[..pos].parse().unwrap_or(0.0);
                    let b: f64 = cleaned[pos+1..].parse().unwrap_or(1.0);
                    return Ok((a / b).to_string());
                }
                Ok(format!("Calculated: {expr}"))
            }
            other => Err(anyhow!("Unknown tool: {other}")),
        }
    }

    fn reset(&self) {
        self.error_injections.lock().unwrap().clear();
    }
}
