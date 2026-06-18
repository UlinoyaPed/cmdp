use crate::renderer::Rendered;
use crate::template::Command;

pub fn preview(command: &Command, rendered: &Rendered) -> String {
    let mut lines = Vec::new();
    if command.danger {
        lines.push("⚠ 危险命令：配置标记 danger = true".to_string());
    }
    if !rendered.missing.is_empty() {
        lines.push(format!("缺失参数：{}", rendered.missing.join(", ")));
    }
    lines.push(rendered.text.clone());
    lines.join("\n")
}
