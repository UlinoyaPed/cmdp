use crate::parser::{Node, ParsedTemplate};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Rendered {
    pub text: String,
    pub missing: Vec<String>,
}

pub fn render(
    tpl: &ParsedTemplate,
    values: &HashMap<String, String>,
    enabled: &HashSet<String>,
    mask_secret: &HashSet<String>,
) -> Rendered {
    let mut missing = Vec::new();
    let mut text = RenderText::default();
    render_nodes(
        &tpl.nodes,
        values,
        enabled,
        mask_secret,
        &mut missing,
        &mut text,
    );
    Rendered {
        text: text.finish(),
        missing,
    }
}

fn render_nodes(
    nodes: &[Node],
    values: &HashMap<String, String>,
    enabled: &HashSet<String>,
    mask: &HashSet<String>,
    missing: &mut Vec<String>,
    out: &mut RenderText,
) {
    for n in nodes {
        match n {
            Node::Text(t) => out.push_template_text(t),
            Node::Placeholder(p) => match values.get(p).filter(|v| !v.is_empty()) {
                Some(_) if mask.contains(p) => out.push_value("******"),
                Some(v) => out.push_value(v),
                None => {
                    push_missing(missing, p);
                    out.push_value(&format!("<{p}?>"));
                }
            },
            Node::Required(body) => render_nodes(body, values, enabled, mask, missing, out),
            Node::Optional { id, body } if enabled.contains(id) => {
                render_nodes(body, values, enabled, mask, missing, out);
            }
            Node::Optional { .. } => {}
        }
    }
}

fn push_missing(missing: &mut Vec<String>, name: &str) {
    if !missing.iter().any(|existing| existing == name) {
        missing.push(name.to_string());
    }
}

#[derive(Default)]
struct RenderText {
    text: String,
    pending_template_space: bool,
}

impl RenderText {
    fn push_template_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch.is_whitespace() {
                self.pending_template_space = true;
            } else {
                self.flush_template_space();
                self.text.push(ch);
            }
        }
    }

    fn push_value(&mut self, value: &str) {
        self.flush_template_space();
        self.text.push_str(value);
    }

    fn finish(self) -> String {
        self.text
    }

    fn flush_template_space(&mut self) {
        if self.pending_template_space {
            if !self.text.is_empty() {
                self.text.push(' ');
            }
            self.pending_template_space = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_template;

    #[test]
    fn disabled_optional_fragments_do_not_require_params() {
        let template = parse_template("rg [[glob:--glob {{glob}}]] <<{{query}}>>").unwrap();
        let values = HashMap::from([("query".to_string(), "needle".to_string())]);
        let enabled = HashSet::new();

        let rendered = render(&template, &values, &enabled, &HashSet::new());

        assert_eq!(rendered.text, "rg needle");
        assert!(rendered.missing.is_empty());
    }

    #[test]
    fn enabled_optional_fragments_require_their_params() {
        let template = parse_template("rg [[glob:--glob {{glob}}]] <<{{query}}>>").unwrap();
        let values = HashMap::from([("query".to_string(), "needle".to_string())]);
        let enabled = HashSet::from(["glob".to_string()]);

        let rendered = render(&template, &values, &enabled, &HashSet::new());

        assert_eq!(rendered.text, "rg --glob <glob?> needle");
        assert_eq!(rendered.missing, vec!["glob"]);
    }

    #[test]
    fn preserves_user_value_whitespace() {
        let template = parse_template("echo <<{{value}}>> [[name:--name {{name}}]]").unwrap();
        let values = HashMap::from([
            ("value".to_string(), "a  b".to_string()),
            ("name".to_string(), "first  last".to_string()),
        ]);
        let enabled = HashSet::from(["name".to_string()]);

        let rendered = render(&template, &values, &enabled, &HashSet::new());

        assert_eq!(rendered.text, "echo a  b --name first  last");
    }
}
