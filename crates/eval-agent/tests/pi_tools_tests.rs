use eval_agent::{get_pi_tools, SimulatedEnvironment, WorkspaceEnv};
use std::collections::HashMap;

#[test]
fn test_pi_tools_workspace_operations() {
    let ws = WorkspaceEnv::new().unwrap();

    // 1. Setup seed files
    let mut files = HashMap::new();
    files.insert(
        "src/calculator.py".to_string(),
        "def add(a, b):\n    return a - b\n".to_string(),
    );
    files.insert(
        "tests/test_calc.py".to_string(),
        "assert True\n".to_string(),
    );
    ws.setup_from_files(&files).unwrap();

    // 2. Test read_file
    let read_res = ws
        .execute_tool(
            "read_file",
            r#"{"path": "src/calculator.py", "offset": 1, "limit": 10}"#,
        )
        .unwrap();
    assert!(read_res.contains("return a - b"));

    // 3. Test grep_search
    let grep_res = ws
        .execute_tool("grep_search", r#"{"query": "def add"}"#)
        .unwrap();
    assert!(grep_res.contains("src/calculator.py"));

    // 4. Test edit_file (Patch bug)
    let edit_res = ws
        .execute_tool(
            "edit_file",
            r#"{"path": "src/calculator.py", "old_str": "return a - b", "new_str": "return a + b"}"#,
        )
        .unwrap();
    assert!(edit_res.contains("Successfully replaced"));

    let content_after = ws.read_file("src/calculator.py").unwrap();
    assert_eq!(content_after, "def add(a, b):\n    return a + b\n");

    // 5. Test write_file
    let write_res = ws
        .execute_tool(
            "write_file",
            r##"{"path": "notes.md", "content": "Header line\nProject Notes"}"##,
        )
        .unwrap();
    assert!(write_res.contains("Successfully written"));
    assert_eq!(
        ws.read_file("notes.md").unwrap(),
        "Header line\nProject Notes"
    );

    // 6. Test bash execution
    let bash_res = ws
        .execute_tool("bash", r#"{"command": "python3 -c 'from src.calculator import add; assert add(2, 3) == 5; print(\"OK\")'"}"#)
        .unwrap();
    assert!(bash_res.contains("OK"));

    // 7. Test verification runner
    let (passed, report) = ws
        .run_verification("python3 -c 'from src.calculator import add; assert add(10, 5) == 15'")
        .unwrap();
    assert!(passed);
    assert!(report.contains("PASSED"));
}

#[test]
fn test_get_pi_tools_definitions() {
    let tools = get_pi_tools();
    assert!(tools.len() >= 5);
    let names: Vec<_> = tools.iter().map(|t| t.function.name.as_str()).collect();
    assert!(names.contains(&"bash"));
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
    assert!(names.contains(&"edit_file"));
    assert!(names.contains(&"grep_search"));
    assert!(names.contains(&"calculator"));
    assert!(names.contains(&"search_web"));
    assert!(names.contains(&"database_query"));
}

#[test]
fn test_real_sqlite_database_query_and_write_rejection() {
    let ws = WorkspaceEnv::new().unwrap();

    let out = ws
        .execute_tool(
            "database_query",
            r#"{"query": "SELECT name, balance FROM users ORDER BY id"}"#,
        )
        .unwrap();
    assert!(out.contains("Alice"), "output={out}");
    assert!(out.contains("5000"), "output={out}");
    assert!(out.contains("columns"), "output={out}");

    let write = ws.execute_tool(
        "database_query",
        r#"{"query": "DELETE FROM users WHERE id = 1"}"#,
    );
    assert!(write.is_err(), "write query must be rejected");

    let missing = ws.execute_tool(
        "database_query",
        r#"{"query": "SELECT * FROM users", "database": "../etc/passwd"}"#,
    );
    assert!(
        missing.is_err(),
        "path traversal database path must be rejected"
    );
}

#[test]
fn test_search_web_requires_operator_endpoint() {
    if std::env::var_os("AGENT_BENCH_SEARCH_ENDPOINT").is_some() {
        return; // Environment explicitly configured; covered by integration use.
    }
    let ws = WorkspaceEnv::new().unwrap();
    let err = ws
        .execute_tool("search_web", r#"{"query": "rust tokio"}"#)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("AGENT_BENCH_SEARCH_ENDPOINT"),
        "unexpected error: {err}"
    );
}
