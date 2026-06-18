use regex::Regex;

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

pub fn parse_template(input: &str) -> Result<ParsedTemplate, String> {
    let nodes = parse_level(input, true)?;
    Ok(ParsedTemplate { nodes })
}

fn parse_level(input: &str, allow_segments: bool) -> Result<Vec<Node>, String> {
    let name_re = Regex::new(r"^[A-Za-z0-9_-]+$").unwrap();
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        if rest.starts_with("{{") {
            let end = rest.find("}}").ok_or("unclosed placeholder")?;
            let name = rest[2..end].trim();
            if !name_re.is_match(name) {
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
            let (id, body) = if let Some((id, body)) = raw.split_once(':') {
                let id = id.trim();
                if !name_re.is_match(id) {
                    return Err(format!("invalid optional id '{id}'"));
                }
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
