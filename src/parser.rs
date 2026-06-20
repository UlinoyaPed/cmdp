#[derive(Debug, Clone)]
pub enum Node {
    Text(String),
    Placeholder(String),
    Required(Vec<Node>),
    Optional { id: String, body: Vec<Node> },
}

#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateUsage {
    pub required_params: Vec<String>,
    pub optional: Vec<OptionalUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalUsage {
    pub id: String,
    pub params: Vec<String>,
}

pub fn parse_template(input: &str) -> Result<ParsedTemplate, String> {
    let nodes = parse_level(input.trim(), true)?;
    Ok(ParsedTemplate { nodes })
}

pub fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn analyze_template(template: &ParsedTemplate) -> TemplateUsage {
    let mut usage = TemplateUsage {
        required_params: Vec::new(),
        optional: Vec::new(),
    };
    for node in &template.nodes {
        match node {
            Node::Placeholder(name) => push_unique(&mut usage.required_params, name),
            Node::Required(body) => collect_placeholders(body, &mut usage.required_params),
            Node::Optional { id, body } => {
                let mut params = Vec::new();
                collect_placeholders(body, &mut params);
                if let Some(existing) = usage.optional.iter_mut().find(|opt| opt.id == *id) {
                    for param in params {
                        push_unique(&mut existing.params, &param);
                    }
                } else {
                    usage.optional.push(OptionalUsage {
                        id: id.clone(),
                        params,
                    });
                }
            }
            Node::Text(_) => {}
        }
    }
    usage
}

fn parse_level(input: &str, allow_segments: bool) -> Result<Vec<Node>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        if rest.starts_with("{{") {
            let end = rest.find("}}").ok_or("unclosed placeholder")?;
            let name = rest[2..end].trim();
            if !is_identifier(name) {
                return Err(format!("invalid placeholder name '{name}'"));
            }
            out.push(Node::Placeholder(name.to_string()));
            i += end + 2;
        } else if rest.starts_with("<<") {
            if !allow_segments {
                return Err("nested required/optional segments are not supported".into());
            }
            let end = rest.find(">>").ok_or("unclosed required segment")?;
            let body = &rest[2..end];
            if body.contains("[[") || body.contains("<<") {
                return Err("nested segments are not supported".into());
            }
            out.push(Node::Required(parse_level(body, false)?));
            i += end + 2;
        } else if rest.starts_with("[[") {
            if !allow_segments {
                return Err("nested required/optional segments are not supported".into());
            }
            let end = rest.find("]]").ok_or("unclosed optional segment")?;
            let raw = &rest[2..end];
            if raw.contains("[[") || raw.contains("<<") {
                return Err("nested segments are not supported".into());
            }
            let (id, body) = if let Some((id, body)) = raw
                .split_once(':')
                .filter(|(id, body)| is_identifier(id.trim()) && !body.starts_with("//"))
            {
                let id = id.trim();
                (id.to_string(), body)
            } else {
                (
                    format!(
                        "option_{}",
                        out.iter()
                            .filter(|n| matches!(n, Node::Optional { .. }))
                            .count()
                            + 1
                    ),
                    raw,
                )
            };
            out.push(Node::Optional {
                id,
                body: parse_level(body, false)?,
            });
            i += end + 2;
        } else {
            let next = [rest.find("{{"), rest.find("<<"), rest.find("[[")]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(rest.len());
            out.push(Node::Text(rest[..next].to_string()));
            i += next;
        }
    }
    Ok(out)
}

fn collect_placeholders(nodes: &[Node], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Placeholder(name) => push_unique(out, name),
            Node::Required(body) | Node::Optional { body, .. } => collect_placeholders(body, out),
            Node::Text(_) => {}
        }
    }
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_required_optional_and_placeholders() {
        let parsed =
            parse_template("cmd <<{{path}}>> [[verbose:-v]] [[glob:--glob {{glob}}]]").unwrap();
        let usage = analyze_template(&parsed);

        assert_eq!(usage.required_params, vec!["path"]);
        assert_eq!(
            usage.optional,
            vec![
                OptionalUsage {
                    id: "verbose".to_string(),
                    params: vec![]
                },
                OptionalUsage {
                    id: "glob".to_string(),
                    params: vec!["glob".to_string()]
                }
            ]
        );
    }

    #[test]
    fn rejects_nested_segments() {
        let err = parse_template("cmd [[flag:<<{{path}}>>]]").unwrap_err();
        assert!(err.contains("nested"));
    }

    #[test]
    fn supports_colons_inside_anonymous_optional_body() {
        let parsed = parse_template("curl [[https://example.test/{{path}}]]").unwrap();
        let usage = analyze_template(&parsed);

        assert_eq!(usage.optional[0].id, "option_1");
        assert_eq!(usage.optional[0].params, vec!["path"]);
    }

    #[test]
    fn treats_shell_redirection_as_template_text() {
        let parsed =
            parse_template(r#"sort < <<"{{input}}">> > <<"{{output}}">> [[append:>> "{{log}}"]]"#)
                .unwrap();
        let usage = analyze_template(&parsed);

        assert_eq!(usage.required_params, vec!["input", "output"]);
        assert_eq!(
            usage.optional,
            vec![OptionalUsage {
                id: "append".to_string(),
                params: vec!["log".to_string()],
            }]
        );
    }
}
