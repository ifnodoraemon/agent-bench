use std::collections::HashMap;

/// A lightweight, strongly-typed prompt template supporting {variable} interpolation.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: &'static str,
    pub template: &'static str,
    pub description: &'static str,
}

impl PromptTemplate {
    pub const fn new(name: &'static str, template: &'static str, description: &'static str) -> Self {
        Self {
            name,
            template,
            description,
        }
    }

    /// Render template by replacing `{key}` placeholders with provided string values.
    pub fn render(&self, vars: &HashMap<&str, &str>) -> String {
        let mut rendered = self.template.to_string();
        for (k, v) in vars {
            let placeholder = format!("{{{k}}}");
            rendered = rendered.replace(&placeholder, v);
        }
        rendered
    }

    /// Convenience render with a static slice of key-value tuples.
    pub fn render_tuples(&self, vars: &[(&str, &str)]) -> String {
        let mut rendered = self.template.to_string();
        for (k, v) in vars {
            let placeholder = format!("{{{k}}}");
            rendered = rendered.replace(&placeholder, v);
        }
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_render() {
        let tpl = PromptTemplate::new(
            "greeting",
            "Hello {user}, welcome to {project}!",
            "Sample greeting prompt",
        );
        let rendered = tpl.render_tuples(&[("user", "Developer"), ("project", "agent-bench")]);
        assert_eq!(rendered, "Hello Developer, welcome to agent-bench!");
    }
}
