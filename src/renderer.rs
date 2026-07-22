use crate::parser::{Node, ParsedTemplate};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Rendered {
    pub text: String,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCommand {
    pub execution_text: String,
    pub display_text: String,
}

#[derive(Debug, Clone)]
pub struct Prepared {
    pub command: PreparedCommand,
    pub missing: Vec<String>,
}

pub fn render(
    tpl: &ParsedTemplate,
    values: &HashMap<String, String>,
    enabled: &HashSet<String>,
    mask_secret: &HashSet<String>,
) -> Rendered {
    render_mode(tpl, values, enabled, mask_secret, false)
}

pub fn prepare(
    tpl: &ParsedTemplate,
    values: &HashMap<String, String>,
    enabled: &HashSet<String>,
    secret: &HashSet<String>,
) -> Prepared {
    let execution = render_mode(tpl, values, enabled, &HashSet::new(), true);
    let display = render_mode(tpl, values, enabled, secret, true);
    Prepared {
        command: PreparedCommand {
            execution_text: execution.text,
            display_text: display.text,
        },
        missing: execution.missing,
    }
}

fn render_mode(
    tpl: &ParsedTemplate,
    values: &HashMap<String, String>,
    enabled: &HashSet<String>,
    mask_secret: &HashSet<String>,
    shell_escape: bool,
) -> Rendered {
    let mut missing = Vec::new();
    let mut text = RenderText::default();
    render_nodes(
        &tpl.nodes,
        values,
        enabled,
        mask_secret,
        shell_escape,
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
    shell_escape: bool,
    missing: &mut Vec<String>,
    out: &mut RenderText,
) {
    for n in nodes {
        match n {
            Node::Text(t) => out.push_template_text(t),
            Node::Placeholder { name: p, raw } => match values.get(p) {
                Some(_) if mask.contains(p) => out.push_value("'******'"),
                Some(v) if shell_escape && !raw => out.push_value(&shell_literal(v)),
                Some(v) => out.push_value(v),
                None => {
                    push_missing(missing, p);
                    out.push_value(&format!("<{p}?>"));
                }
            },
            Node::Required(body) => {
                render_nodes(body, values, enabled, mask, shell_escape, missing, out)
            }
            Node::Optional { id, body } if enabled.contains(id) => {
                render_nodes(body, values, enabled, mask, shell_escape, missing, out);
            }
            Node::Optional { .. } => {}
        }
    }
}

fn shell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
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

        assert_eq!(rendered.text, r#"sort < in.txt > out.txt 2>> cmd.log"#);
        assert!(rendered.missing.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn safe_parameters_are_executed_as_literal_data() {
        use std::{
            fs,
            process::Command,
            time::{SystemTime, UNIX_EPOCH},
        };
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cmdp-shell-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        let output = dir.join("out; special.txt");
        let marker = dir.join("injected");
        let cases = [
            "has spaces",
            "a'b",
            "a\"b",
            "$(printf injected)",
            "`printf injected`",
            "value; printf nope",
            "line one\nline two",
            "",
        ];
        let template = parse_template("printf '%s' {{value}} > {{output}}").unwrap();
        for value in cases {
            let values = HashMap::from([
                ("value".to_string(), value.to_string()),
                ("output".to_string(), output.display().to_string()),
            ]);
            let prepared = prepare(&template, &values, &HashSet::new(), &HashSet::new());
            let status = Command::new("/bin/sh")
                .arg("-c")
                .arg(&prepared.command.execution_text)
                .status()
                .unwrap();
            assert!(status.success());
            assert_eq!(fs::read_to_string(&output).unwrap(), value);
            assert!(!marker.exists());
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn command_substitution_cannot_create_a_file() {
        use std::{
            process::Command,
            time::{SystemTime, UNIX_EPOCH},
        };
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let marker = std::env::temp_dir().join(format!("cmdp-marker-{nonce}"));
        let template = parse_template("printf '%s' {{value}} > /dev/null").unwrap();
        let value = format!("$(touch {})", marker.display());
        let prepared = prepare(
            &template,
            &HashMap::from([("value".into(), value.clone())]),
            &HashSet::new(),
            &HashSet::new(),
        );
        assert!(
            Command::new("/bin/sh")
                .arg("-c")
                .arg(&prepared.command.execution_text)
                .status()
                .unwrap()
                .success()
        );
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[test]
    fn raw_parameters_preserve_explicit_shell_semantics() {
        use std::process::Command;
        let template = parse_template("test {{{expression}}}").unwrap();
        let prepared = prepare(
            &template,
            &HashMap::from([("expression".into(), "1 -eq 1".into())]),
            &HashSet::new(),
            &HashSet::new(),
        );
        assert!(
            Command::new("/bin/sh")
                .arg("-c")
                .arg(&prepared.command.execution_text)
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    fn secret_values_only_appear_in_execution_text() {
        let template = parse_template("login --token {{token}}").unwrap();
        let prepared = prepare(
            &template,
            &HashMap::from([("token".into(), "very-secret".into())]),
            &HashSet::new(),
            &HashSet::from(["token".into()]),
        );
        assert!(prepared.command.execution_text.contains("very-secret"));
        assert!(!prepared.command.display_text.contains("very-secret"));
        assert!(prepared.command.display_text.contains("******"));
    }

    #[test]
    fn legacy_double_quoted_placeholders_are_safely_normalized() {
        let template = parse_template("cat <<\"{{path}}\">>").unwrap();
        let prepared = prepare(
            &template,
            &HashMap::from([("path".into(), "a b".into())]),
            &HashSet::new(),
            &HashSet::new(),
        );
        assert_eq!(prepared.command.execution_text, "cat 'a b'");
    }
}
