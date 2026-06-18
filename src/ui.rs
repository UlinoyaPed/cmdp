use crate::app::{App, Focus};
use ratatui::{prelude::*, widgets::*};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(7),
        ])
        .split(f.size());
    let title = format!(
        " cmdp | sources: {} | mode: {} ",
        app.config.sources.join(", "),
        if app.editing { "editing" } else { "normal" }
    );
    f.render_widget(
        Paragraph::new(title).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(30),
            Constraint::Percentage(45),
        ])
        .split(chunks[1]);
    draw_categories(f, app, cols[0]);
    draw_commands(f, app, cols[1]);
    draw_form(f, app, cols[2]);
    let mut p = app.preview_text();
    if let Some(e) = &app.error {
        p = format!("{e}\n{p}");
    }
    f.render_widget(
        Paragraph::new(p)
            .wrap(Wrap { trim: false })
            .block(block("预览 Ctrl+y 输出", app.focus == Focus::Preview)),
        chunks[2],
    );
}
fn block(t: &str, focus: bool) -> Block<'static> {
    Block::default()
        .title(t.to_string())
        .borders(Borders::ALL)
        .border_style(if focus {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        })
}
fn draw_categories(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .config
        .categories
        .iter()
        .map(|(id, c)| ListItem::new(format!("{} ({})", c.alias.as_deref().unwrap_or(id), id)))
        .collect();
    let mut st = ListState::default();
    st.select(Some(app.category_idx));
    f.render_stateful_widget(
        List::new(items)
            .highlight_symbol("> ")
            .block(block("分类", app.focus == Focus::Categories)),
        area,
        &mut st,
    );
}
fn draw_commands(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .commands_in_category()
        .into_iter()
        .map(|(_, c)| {
            let desc = c.description.as_deref().unwrap_or("");
            ListItem::new(format!(
                "[{}] {}{} {}",
                c.source.label(),
                if c.danger { "⚠ " } else { "" },
                c.title.as_deref().unwrap_or("未命名"),
                desc
            ))
        })
        .collect();
    let mut st = ListState::default();
    st.select(Some(app.command_idx));
    f.render_stateful_widget(
        List::new(items)
            .highlight_symbol("> ")
            .block(block("命令", app.focus == Focus::Commands)),
        area,
        &mut st,
    );
}
fn draw_form(f: &mut Frame, app: &App, area: Rect) {
    let mut rows = Vec::new();
    if let Some((_, cmd)) = app.current_command() {
        for p in &cmd.params {
            let v = app.values.get(&p.name).cloned().unwrap_or_default();
            rows.push(format!(
                "{}: {}{}",
                p.label.as_deref().unwrap_or(&p.name),
                if p.secret && !v.is_empty() {
                    "******".into()
                } else {
                    v
                },
                p.help
                    .as_ref()
                    .or(p.placeholder.as_ref())
                    .map(|x| format!(" ({x})"))
                    .unwrap_or_default()
            ));
        }
        for o in &cmd.options {
            rows.push(format!(
                "[{}] {}",
                if app.enabled.contains(&o.id) {
                    "x"
                } else {
                    " "
                },
                o.label.as_deref().unwrap_or(&o.id)
            ));
        }
    }
    if app.editing {
        rows.push(format!("编辑: {}", app.edit_buffer));
    }
    let items: Vec<ListItem> = rows.into_iter().map(ListItem::new).collect();
    let mut st = ListState::default();
    st.select(Some(app.form_idx));
    f.render_stateful_widget(
        List::new(items)
            .highlight_symbol("> ")
            .block(block("参数 / 选项", app.focus == Focus::Form)),
        area,
        &mut st,
    );
}
