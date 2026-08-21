pub mod agent;
pub mod judge;
pub mod safety;
pub mod template;

pub use agent::*;
pub use judge::*;
pub use safety::*;
pub use template::PromptTemplate;

use std::collections::HashMap;

/// Centralized Prompt Catalog providing unified lookup, dynamic overrides, and templating.
#[derive(Debug, Clone, Default)]
pub struct PromptCatalog {
    custom_overrides: HashMap<String, String>,
}

impl PromptCatalog {
    pub fn new() -> Self {
        Self {
            custom_overrides: HashMap::new(),
        }
    }

    /// Register a runtime override for a prompt template name
    pub fn override_prompt(&mut self, name: impl Into<String>, template: impl Into<String>) {
        self.custom_overrides.insert(name.into(), template.into());
    }

    /// Render a prompt template with variables, respecting any registered overrides
    pub fn render(&self, default_tpl: &PromptTemplate, vars: &HashMap<&str, &str>) -> String {
        if let Some(custom) = self.custom_overrides.get(default_tpl.name) {
            let mut rendered = custom.clone();
            for (k, v) in vars {
                let placeholder = format!("{{{k}}}");
                rendered = rendered.replace(&placeholder, v);
            }
            rendered
        } else {
            default_tpl.render(vars)
        }
    }

    /// Render a prompt template with variable tuples, respecting any registered overrides
    pub fn render_tuples(&self, default_tpl: &PromptTemplate, vars: &[(&str, &str)]) -> String {
        if let Some(custom) = self.custom_overrides.get(default_tpl.name) {
            let mut rendered = custom.clone();
            for (k, v) in vars {
                let placeholder = format!("{{{k}}}");
                rendered = rendered.replace(&placeholder, v);
            }
            rendered
        } else {
            default_tpl.render_tuples(vars)
        }
    }
}
