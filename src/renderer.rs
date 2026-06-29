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
    pending_template_newlines: usize,
    pending_line_indent: String,
}

impl RenderText {
    fn push_template_text(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    self.queue_template_newline();
                }
                '\n' => self.queue_template_newline(),
                ' ' | '\t' => self.queue_template_space(ch),
                ch if ch.is_whitespace() => self.queue_template_space(' '),
                _ => {
                    self.flush_template_spacing();
                    self.text.push(ch);
                }
            }
        }
    }

    fn push_value(&mut self, value: &str) {
        self.flush_template_spacing();
        self.text.push_str(value);
    }

    fn finish(self) -> String {
        self.text
    }

    fn queue_template_newline(&mut self) {
        self.pending_template_newlines += 1;
        self.pending_template_space = false;
        self.pending_line_indent.clear();
    }

    fn queue_template_space(&mut self, ch: char) {
        if self.pending_template_newlines > 0 {
            self.pending_line_indent.push(ch);
        } else {
            self.pending_template_space = true;
        }
    }

    fn flush_template_spacing(&mut self) {
        if self.pending_template_newlines > 0 {
            if !self.text.is_empty() {
                for _ in 0..self.pending_template_newlines {
                    self.text.push('\n');
                }
                self.text.push_str(&self.pending_line_indent);
            }
            self.pending_template_newlines = 0;
            self.pending_line_indent.clear();
            return;
        }
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

    #[test]
    fn preserves_template_newlines_between_commands() {
        let template = parse_template("echo one\necho two").unwrap();

        let rendered = render(&template, &HashMap::new(), &HashSet::new(), &HashSet::new());

        assert_eq!(rendered.text, "echo one\necho two");
        assert!(rendered.missing.is_empty());
    }

    #[test]
    fn preserves_multiline_template_indent_before_values() {
        let template = parse_template("if true; then\n  echo <<{{value}}>>\nfi").unwrap();
        let values = HashMap::from([("value".to_string(), "ok".to_string())]);

        let rendered = render(&template, &values, &HashSet::new(), &HashSet::new());

        assert_eq!(rendered.text, "if true; then\n  echo ok\nfi");
        assert!(rendered.missing.is_empty());
    }

    #[test]
    fn preserves_shell_redirection_operators() {
        let template =
            parse_template(r#"sort < <<"{{input}}">> > <<"{{output}}">> [[append:2>> "{{log}}"]]"#)
                .unwrap();
        let values = HashMap::from([
            ("input".to_string(), "in.txt".to_string()),
            ("output".to_string(), "out.txt".to_string()),
            ("log".to_string(), "cmd.log".to_string()),
        ]);
        let enabled = HashSet::from(["append".to_string()]);

        let rendered = render(&template, &values, &enabled, &HashSet::new());

        assert_eq!(
            rendered.text,
            r#"sort < "in.txt" > "out.txt" 2>> "cmd.log""#
        );
        assert!(rendered.missing.is_empty());
    }
}
