use super::template::PromptTemplate;

/// Standard autonomous software engineering agent system prompt.
pub const AGENT_DEFAULT_SYSTEM: PromptTemplate = PromptTemplate::new(
    "agent_default_system",
    "You are an expert autonomous AI software engineer and system administrator operating in the root directory of an isolated project workspace. You have access to tools (bash, read_file, write_file, edit_file, grep_search) to inspect the workspace, execute commands, modify files, and verify results. Always use relative paths when accessing files in the workspace.",
    "Default system prompt for autonomous agent workspace execution",
);

/// Final synthesis prompt injected when reaching maximum allowed turns.
pub const AGENT_FINAL_SYNTHESIS: PromptTemplate = PromptTemplate::new(
    "agent_final_synthesis",
    "This is your final interaction turn. Do not call any further tools. Please synthesize all gathered evidence, data, and findings to output your complete final deliverable / report as requested in the initial instructions.",
    "Prompt to guide agent to summarize and produce final output on the last turn",
);

/// Agent error recovery prompt when a tool execution fails.
pub const AGENT_TOOL_ERROR_RECOVERY: PromptTemplate = PromptTemplate::new(
    "agent_tool_error_recovery",
    "The tool execution for `{tool_name}` failed with error: `{error_message}`. Please inspect the failure and choose an alternative approach to complete the task.",
    "Prompt injected when a tool invocation returns an unexpected error",
);
