use crate::i18n::Texts;
use crate::renderer::Rendered;
use crate::template::Command;

pub fn preview(command: &Command, rendered: &Rendered, texts: &Texts) -> String {
    let mut lines = Vec::new();
    if command.danger {
        lines.push(texts.danger_preview.to_string());
    }
    if !rendered.missing.is_empty() {
        lines.push(format!(
            "{}{}",
            texts.missing_params_prefix,
            rendered.missing.join(", ")
        ));
    }
    lines.push(rendered.text.clone());
    lines.join("\n")
}
