use crate::{
    app::{App, Focus, FormItem},
    template::Source,
};
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

    draw_header(f, app, chunks[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(28),
            Constraint::Percentage(48),
        ])
        .split(chunks[1]);
    draw_categories(f, app, cols[0]);
    draw_commands(f, app, cols[1]);
    draw_form(f, app, cols[2]);

    let mut preview = app.preview_text();
    if let Some(error) = &app.error {
        preview = format!("{error}\n{preview}");
    }
    f.render_widget(
        Paragraph::new(preview)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
            .block(
                block("预览  Ctrl+y 输出", false).border_style(Style::default().fg(Color::Blue)),
            ),
        chunks[2],
    );
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mode = if app.editing {
        "编辑参数"
    } else if app.search_editing {
        "搜索命令"
    } else {
        "普通"
    };
    let search = if app.search_active() {
        format!("/{}", truncate(&app.search_query, 24))
    } else {
        "/ 搜索".to_string()
    };
    let line = Line::from(vec![
        Span::styled(
            " cmdp ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {mode} "), Style::default().fg(Color::Yellow)),
        Span::styled(
            format!(" cfg:{} ", source_summary(&app.config.sources)),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(
            format!(" {search} "),
            if app.search_editing {
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
        ),
        Span::styled(
            " Tab切换  Ctrl+y输出  q退出 ",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        area,
    );
}

fn block(t: &str, focus: bool) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            t.to_string(),
            Style::default().fg(if focus { Color::Yellow } else { Color::Gray }),
        ))
        .borders(Borders::ALL)
        .border_style(if focus {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        })
}

fn draw_categories(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .config
        .categories
        .iter()
        .map(|(id, c)| {
            ListItem::new(Line::from(vec![
                Span::raw(c.alias.as_deref().unwrap_or(id)),
                Span::styled(format!(" ({id})"), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.category_idx));
    f.render_stateful_widget(
        List::new(items)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .block(block("分类", app.focus == Focus::Categories)),
        area,
        &mut state,
    );
}

fn draw_commands(f: &mut Frame, app: &App, area: Rect) {
    let commands = app.visible_commands();
    let items: Vec<ListItem> = if commands.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            if app.search_active() {
                "无匹配命令"
            } else {
                "无命令"
            },
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        commands
            .into_iter()
            .map(|(id, command)| command_item(app, id, command))
            .collect()
    };

    let mut state = ListState::default();
    if app.visible_commands().is_empty() {
        state.select(None);
    } else {
        state.select(Some(app.command_idx));
    }

    let title = if app.search_active() {
        format!("命令  /{}", truncate(&app.search_query, 16))
    } else {
        "命令".to_string()
    };
    f.render_stateful_widget(
        List::new(items)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .block(block(
                &title,
                app.focus == Focus::Commands || app.search_editing,
            )),
        area,
        &mut state,
    );
}

fn command_item(app: &App, id: &str, command: &crate::template::Command) -> ListItem<'static> {
    let mut spans = vec![
        Span::styled(
            format!("[{}] ", command.source.label()),
            source_style(command.source),
        ),
        Span::styled(
            if command.danger { "! " } else { "" },
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            command.title.as_deref().unwrap_or(id).to_string(),
            Style::default().fg(Color::White),
        ),
    ];
    if app.search_active() {
        spans.push(Span::styled(
            format!(" ({})", command.category),
            Style::default().fg(Color::DarkGray),
        ));
    }
    ListItem::new(Line::from(spans))
}

fn draw_form(f: &mut Frame, app: &App, area: Rect) {
    let form_items = app.form_items();
    let mut rows: Vec<ListItem> = Vec::new();

    for (idx, item) in form_items.into_iter().enumerate() {
        let selected = app.focus == Focus::Form && idx == app.form_idx;
        rows.push(match item {
            FormItem::Param {
                label,
                value,
                placeholder,
                help,
                secret,
                choices,
                required,
                ..
            } => param_item(
                app,
                selected,
                label,
                value,
                placeholder,
                help,
                secret,
                choices,
                required,
            ),
            FormItem::Option { label, enabled, .. } => option_item(selected, label, enabled),
        });
    }

    if rows.is_empty() && app.current_command().is_some() {
        rows.push(ListItem::new(Line::from(Span::styled(
            "无参数或可选项",
            Style::default().fg(Color::DarkGray),
        ))));
    }

    let mut state = ListState::default();
    if rows.is_empty() {
        state.select(None);
    } else {
        state.select(Some(app.form_idx));
    }
    f.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .block(block("参数 / 选项", app.focus == Focus::Form)),
        area,
        &mut state,
    );
}

#[allow(clippy::too_many_arguments)]
fn param_item(
    app: &App,
    selected: bool,
    label: String,
    value: String,
    placeholder: Option<String>,
    help: Option<String>,
    secret: bool,
    choices: Vec<String>,
    required: bool,
) -> ListItem<'static> {
    let editing = app.editing && selected;
    let raw_value = if editing {
        format!("{}|", app.edit_buffer)
    } else if secret && !value.is_empty() {
        "******".to_string()
    } else {
        value
    };
    let empty = raw_value.is_empty();
    let display_value = if empty {
        placeholder.clone().unwrap_or_else(|| "输入...".to_string())
    } else {
        raw_value
    };
    let input_style = if empty {
        Style::default().fg(Color::DarkGray)
    } else if editing {
        Style::default().fg(Color::Black).bg(Color::LightYellow)
    } else {
        Style::default().fg(Color::White)
    };
    let input_edge_style = if selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let detail = help
        .or(placeholder)
        .map(|text| format!("  {text}"))
        .unwrap_or_default();
    let choices_hint = if choices.is_empty() {
        String::new()
    } else {
        format!("  {}", choices.join(" / "))
    };

    ListItem::new(Line::from(vec![
        Span::styled(
            if required { "* " } else { "  " },
            Style::default().fg(if required {
                Color::Red
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled(
            label,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("[ ", input_edge_style),
        Span::styled(display_value, input_style),
        Span::styled(" ]", input_edge_style),
        Span::styled(choices_hint, Style::default().fg(Color::LightBlue)),
        Span::styled(detail, Style::default().fg(Color::DarkGray)),
    ]))
}

fn option_item(selected: bool, label: String, enabled: bool) -> ListItem<'static> {
    let edge_style = if selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    ListItem::new(Line::from(vec![
        Span::styled("[", edge_style),
        Span::styled(
            if enabled { "x" } else { " " },
            Style::default().fg(if enabled {
                Color::LightGreen
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled("] ", edge_style),
        Span::styled(label, Style::default().fg(Color::White)),
    ]))
}

fn source_style(source: Source) -> Style {
    match source {
        Source::Global => Style::default().fg(Color::LightBlue),
        Source::Local => Style::default().fg(Color::LightGreen),
    }
}

fn source_summary(sources: &[String]) -> &'static str {
    let has_global = sources.iter().any(|source| source.starts_with("global:"));
    let has_local = sources.iter().any(|source| source.starts_with("local:"));
    match (has_global, has_local) {
        (true, true) => "global+local",
        (true, false) => "global",
        (false, true) => "local",
        (false, false) => "none",
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}
