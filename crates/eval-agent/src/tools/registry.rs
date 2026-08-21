use eval_core::model::ToolDefinition;
use std::collections::HashMap;

/// Predefined bundle of tools for specific evaluation domains
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolBundle {
    /// SWE & Pi coding toolset (bash, read_file, write_file, edit_file, grep_search)
    PiCoding,
    /// Standard mock toolset (bash, search_web, query_database, calculator)
    StandardMock,
    /// Database operations toolset
    DatabaseOps,
    /// Web research toolset
    WebResearch,
}

/// Pluggable registry for dynamically assembling toolsets
#[derive(Default, Clone)]
pub struct ToolRegistry {
    bundles: HashMap<String, Vec<ToolDefinition>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            bundles: HashMap::new(),
        };
        reg.register_bundle("pi_coding", crate::pi_tools::get_pi_tools());
        reg.register_bundle("standard_mock", super::get_standard_mock_tools());
        reg
    }

    /// Register a custom named bundle of tools
    pub fn register_bundle(&mut self, name: impl Into<String>, tools: Vec<ToolDefinition>) {
        self.bundles.insert(name.into(), tools);
    }

    /// Get tools for a specific bundle name
    pub fn get_bundle(&self, name: &str) -> Option<&Vec<ToolDefinition>> {
        self.bundles.get(name)
    }

    /// Assemble tools from multiple bundle names
    pub fn assemble_bundles(&self, bundle_names: &[&str]) -> Vec<ToolDefinition> {
        let mut assembled = Vec::new();
        for name in bundle_names {
            if let Some(tools) = self.bundles.get(*name) {
                for t in tools {
                    if !assembled.iter().any(|existing: &ToolDefinition| existing.function.name == t.function.name) {
                        assembled.push(t.clone());
                    }
                }
            }
        }
        assembled
    }
}
