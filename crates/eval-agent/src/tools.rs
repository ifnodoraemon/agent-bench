use eval_core::model::ToolDefinition;
use serde_json::json;

pub fn get_standard_mock_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::function(
            "bash",
            "Execute a bash shell command in the environment and return the output.",
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line string to execute, e.g. 'ls -la /var/log' or 'cat config.json'"
                    }
                },
                "required": ["command"]
            }),
        ),
        ToolDefinition::function(
            "search_web",
            "Search the web for up-to-date information, documentation, or facts.",
            json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query string"
                    }
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::function(
            "query_database",
            "Execute a SQL query against the application database.",
            json!({
                "type": "object",
                "properties": {
                    "sql": {
                        "type": "string",
                        "description": "The SQL query to run (e.g. 'SELECT * FROM users WHERE status = 1')"
                    }
                },
                "required": ["sql"]
            }),
        ),
        ToolDefinition::function(
            "calculator",
            "Evaluate a mathematical expression and return the exact numeric result.",
            json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate, e.g. '125 * 34 + sqrt(144)'"
                    }
                },
                "required": ["expression"]
            }),
        ),
    ]
}
