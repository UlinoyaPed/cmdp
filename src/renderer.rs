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
    let text = render_nodes(&tpl.nodes, values, enabled, mask_secret, &mut missing);
    Rendered {
        text: normalize(&text),
        missing,
    }
}

fn render_nodes(
    nodes: &[Node],
    values: &HashMap<String, String>,
    enabled: &HashSet<String>,
    mask: &HashSet<String>,
    missing: &mut Vec<String>,
) -> String {
    let mut s = String::new();
    for n in nodes {
        match n {
            Node::Text(t) => s.push_str(t),
            Node::Placeholder(p) => match values.get(p).filter(|v| !v.is_empty()) {
                Some(_) if mask.contains(p) => s.push_str("******"),
                Some(v) => s.push_str(v),
                None => {
                    missing.push(p.clone());
                    s.push_str(&format!("<{p}?>"));
                }
            },
            Node::Required(body) => s.push_str(&render_nodes(body, values, enabled, mask, missing)),
            Node::Optional { id, body } if enabled.contains(id) => {
                s.push_str(&render_nodes(body, values, enabled, mask, missing))
            }
            Node::Optional { .. } => {}
        }
    }
    s
}

fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
