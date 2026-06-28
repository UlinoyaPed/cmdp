use crate::{
    app::{App, Focus, FormItem},
    i18n::Texts,
    template::Source,
};
use ratatui::{prelude::*, widgets::*};
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub struct UiAreas {
    pub header: Rect,
    pub execute_button: Rect,
    pub categories: Rect,
    pub commands: Rect,
    pub form: Rect,
    pub preview: Rect,
}

pub fn areas(size: Rect) -> UiAreas {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(7),
        ])
        .split(size);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(28),
            Constraint::Percentage(48),
        ])
        .split(chunks[1]);

    UiAreas {
        header: chunks[0],
        execute_button: execute_button_area(chunks[0]),
        categories: cols[0],
        commands: cols[1],
        form: cols[2],
        preview: chunks[2],
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let areas = areas(f.area());
    let texts = app.texts();

    draw_header(f, app, areas.header);
    draw_execute_button(f, app, areas.execute_button);
    draw_categories(f, app, areas.categories);
    draw_commands(f, app, areas.commands);
    draw_form(f, app, areas.form);

    let mut preview = app.preview_text();
    if let Some(error) = &app.error {
        preview = format!("{error}\n{preview}");
    }
    f.render_widget(
        Paragraph::new(preview)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
            .block(
                block(texts.preview_title, false).border_style(Style::default().fg(Color::Blue)),
            ),
        areas.preview,
    );

    if app.show_help {
        draw_help_popup(f, f.area(), texts);
    }
    if app.show_settings {
        draw_settings_popup(f, app, f.area());
    }
    if app.config_editor.is_some() {
        draw_config_editor_popup(f, app, f.area());
    }
    if app.file_picker.is_some() {
        draw_file_picker_popup(f, app, f.area());
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let texts = app.texts();
    let mode = if app.editing {
        texts.mode_editing
    } else if app.search_editing {
        texts.mode_search
    } else {
        texts.mode_normal
    };
    let search = if app.search_active() {
        format!("/{}", truncate(&app.search_query, 24))
    } else {
        texts.search_label.to_string()
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
            format!(" cfg:{} ", source_summary(&app.config.sources, texts)),
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
        Span::styled(texts.header_shortcuts, Style::default().fg(Color::DarkGray)),
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

fn draw_execute_button(f: &mut Frame, app: &App, area: Rect) {
    let texts = app.texts();
    let pending = app.danger_confirmation.is_some();
    let (label, style) = if pending {
        (
            texts.confirm_label,
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            texts.execute_label,
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
    };
    f.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(style),
        area,
    );
}

fn draw_help_popup(f: &mut Frame, area: Rect, texts: &Texts) {
    let popup = centered_rect(area, 72, 20);
    let rows = vec![
        Line::from(vec![
            Span::styled("F1 / ?", key_style()),
            Span::raw(texts.help_toggle),
        ]),
        Line::from(vec![
            Span::styled("Esc", key_style()),
            Span::raw(texts.help_close_popup_or_search),
        ]),
        Line::from(vec![
            Span::styled("q", key_style()),
            Span::raw(texts.help_quit),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab / Shift+Tab", key_style()),
            Span::raw(texts.help_switch_area),
        ]),
        Line::from(vec![
            Span::styled("Left / Right", key_style()),
            Span::raw(texts.help_switch_area),
        ]),
        Line::from(vec![
            Span::styled("Up / Down / j / k", key_style()),
            Span::raw(texts.help_move_selection),
        ]),
        Line::from(vec![
            Span::styled("Enter", key_style()),
            Span::raw(texts.help_enter),
        ]),
        Line::from(vec![
            Span::styled("Space", key_style()),
            Span::raw(texts.help_space),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("/", key_style()),
            Span::raw(texts.help_search),
        ]),
        Line::from(vec![
            Span::styled("F2", key_style()),
            Span::raw(texts.help_settings),
        ]),
        Line::from(vec![
            Span::styled("F3", key_style()),
            Span::raw(texts.help_config_editor),
        ]),
        Line::from(vec![
            Span::styled("f", key_style()),
            Span::raw(texts.help_file_picker),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+d", key_style()),
            Span::raw(texts.help_reset_defaults),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+r", key_style()),
            Span::raw(texts.help_reload_config),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+y", key_style()),
            Span::raw(texts.help_run_current),
        ]),
    ];

    f.render_widget(Clear, popup);
    f.render_widget(
        Paragraph::new(rows)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(Span::styled(
                        texts.help_title,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        popup,
    );
}

fn key_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::LightYellow)
        .add_modifier(Modifier::BOLD)
}

fn draw_settings_popup(f: &mut Frame, app: &App, area: Rect) {
    let texts = app.texts();
    let settings = &app.config.settings;
    let popup = settings_popup_area(area);
    let rows = vec![
        settings_item(texts.settings_language, settings.language.code()),
        settings_item(
            texts.settings_remember_selection,
            bool_label(settings.remember_last_selection, texts),
        ),
        settings_item(
            texts.settings_remember_input,
            bool_label(settings.remember_last_input, texts),
        ),
        settings_item(
            texts.settings_input_record_limit,
            &settings.input_record_limit.to_string(),
        ),
    ];
    let mut state = ListState::default();
    state.select(Some(app.settings_idx));

    f.render_widget(Clear, popup);
    f.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .block(
                Block::default()
                    .title(Span::styled(
                        texts.settings_title,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .title_bottom(Span::styled(
                        texts.settings_help,
                        Style::default().fg(Color::DarkGray),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        popup,
        &mut state,
    );
}

fn settings_item(label: &str, value: &str) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(value.to_string(), Style::default().fg(Color::LightGreen)),
    ]))
}

fn bool_label(value: bool, texts: &Texts) -> &'static str {
    if value {
        texts.settings_on
    } else {
        texts.settings_off
    }
}

fn draw_config_editor_popup(f: &mut Frame, app: &App, area: Rect) {
    let Some(editor) = &app.config_editor else {
        return;
    };
    let texts = app.texts();
    let popup = config_editor_popup_area(area);
    let rows = vec![
        config_editor_item(
            texts.config_editor_command_id,
            editor_value(editor, 0),
            editor.selected == 0 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_category_id,
            editor_value(editor, 1),
            editor.selected == 1 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_category_alias,
            editor_value(editor, 2),
            editor.selected == 2 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_command_title,
            editor_value(editor, 3),
            editor.selected == 3 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_description,
            editor_value(editor, 4),
            editor.selected == 4 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_danger,
            editor_value(editor, 5),
            editor.selected == 5 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_template,
            editor_value(editor, 6),
            editor.selected == 6 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_params,
            editor_value(editor, 7),
            editor.selected == 7 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_options,
            editor_value(editor, 8),
            editor.selected == 8 && editor.editing,
        ),
    ];
    let mut state = ListState::default();
    state.select(Some(editor.selected));

    f.render_widget(Clear, popup);
    f.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .block(
                Block::default()
                    .title(Span::styled(
                        texts.config_editor_title,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .title_bottom(Span::styled(
                        texts.config_editor_help,
                        Style::default().fg(Color::DarkGray),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        popup,
        &mut state,
    );
}

fn editor_value(editor: &crate::app::ConfigEditor, idx: usize) -> String {
    if editor.selected == idx && editor.editing {
        edit_display(&editor.edit_buffer, editor.edit_cursor)
    } else {
        truncate(editor.draft.field(idx), 56)
    }
}

fn config_editor_item(label: &str, value: String, editing: bool) -> ListItem<'static> {
    let value_style = if editing {
        Style::default().fg(Color::Black).bg(Color::LightYellow)
    } else {
        Style::default().fg(Color::LightGreen)
    };
    ListItem::new(Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(value, value_style),
    ]))
}

fn draw_file_picker_popup(f: &mut Frame, app: &App, area: Rect) {
    let Some(picker) = &app.file_picker else {
        return;
    };
    let texts = app.texts();
    let popup = file_picker_popup_area(area);
    let chunks = file_picker_chunks(popup);
    let title = format!(
        "{}{} ",
        texts.file_picker_title_prefix,
        truncate(&picker.dir.display().to_string(), 54)
    );

    f.render_widget(Clear, popup);
    f.render_widget(
        Block::default()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_bottom(Span::styled(
                texts.file_picker_help,
                Style::default().fg(Color::DarkGray),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
        popup,
    );

    let target = app
        .form_items()
        .get(app.form_idx)
        .and_then(|item| match item {
            FormItem::Param { label, .. } => Some(label.clone()),
            FormItem::Option { .. } => None,
        })
        .unwrap_or_else(|| picker.param_name.clone());
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(texts.target_parameter, Style::default().fg(Color::DarkGray)),
            Span::styled(target, Style::default().fg(Color::LightGreen)),
        ])),
        chunks[0],
    );

    if let Some(error) = &picker.error {
        f.render_widget(
            Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: false }),
            chunks[1],
        );
        return;
    }

    let rows: Vec<_> = if picker.entries.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            texts.empty_directory,
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        picker
            .entries
            .iter()
            .map(|entry| {
                let icon = if entry.is_dir { "d " } else { "f " };
                let name = if entry.name == "." {
                    "./".to_string()
                } else if entry.is_dir {
                    format!("{}/", entry.name)
                } else {
                    entry.name.clone()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        icon,
                        Style::default().fg(if entry.is_dir {
                            Color::LightBlue
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::styled(name, Style::default().fg(Color::White)),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();
    if picker.entries.is_empty() {
        state.select(None);
    } else {
        state.select(Some(picker.selected));
    }
    f.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White)),
        chunks[1],
        &mut state,
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
    let texts = app.texts();
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
            .block(block(
                texts.categories_title,
                app.focus == Focus::Categories,
            )),
        area,
        &mut state,
    );
}

fn draw_commands(f: &mut Frame, app: &App, area: Rect) {
    let texts = app.texts();
    let commands = app.visible_commands();
    let items: Vec<ListItem> = if commands.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            if app.search_active() {
                texts.no_matching_commands
            } else if app.config.commands.is_empty() {
                texts.config_not_loaded
            } else {
                texts.no_commands
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
        format!(
            "{}  /{}",
            texts.commands_title,
            truncate(&app.search_query, 16)
        )
    } else {
        texts.commands_title.to_string()
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
            format!("[{}] ", source_short_label(command.source)),
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
    let texts = app.texts();
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
                texts,
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
            texts.no_params_or_options,
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
            .block(block(texts.form_title, app.focus == Focus::Form)),
        area,
        &mut state,
    );
}

#[allow(clippy::too_many_arguments)]
fn param_item(
    texts: &Texts,
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
        edit_display(&app.edit_buffer, app.edit_cursor)
    } else if secret && !value.is_empty() {
        "******".to_string()
    } else {
        value
    };
    let empty = raw_value.is_empty();
    let display_value = if empty {
        placeholder
            .clone()
            .unwrap_or_else(|| texts.input_placeholder.to_string())
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

fn source_short_label(source: Source) -> &'static str {
    match source {
        Source::Global => "g",
        Source::Local => "l",
    }
}

fn source_style(source: Source) -> Style {
    match source {
        Source::Global => Style::default().fg(Color::LightBlue),
        Source::Local => Style::default().fg(Color::LightGreen),
    }
}

fn edit_display(value: &str, cursor: usize) -> String {
    let cursor = cursor.min(value.chars().count());
    let mut display = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if idx == cursor {
            display.push('▌');
        }
        display.push(ch);
    }
    if cursor == value.chars().count() {
        display.push('▌');
    }
    display
}

fn source_summary(sources: &[String], texts: &Texts) -> &'static str {
    let has_global = sources.iter().any(|source| source.starts_with("global:"));
    let has_local = sources.iter().any(|source| source.starts_with("local:"));
    match (has_global, has_local) {
        (true, true) => texts.source_global_local,
        (true, false) => texts.source_global,
        (false, true) => texts.source_local,
        (false, false) => texts.source_none,
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

fn execute_button_area(header: Rect) -> Rect {
    let width = 8.min(header.width.saturating_sub(2));
    Rect {
        x: header
            .x
            .saturating_add(header.width.saturating_sub(width + 2)),
        y: header.y.saturating_add(1),
        width,
        height: 1,
    }
}

fn centered_rect(area: Rect, max_width: u16, max_height: u16) -> Rect {
    let width = max_width.min(area.width.saturating_sub(2)).max(1);
    let height = max_height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

pub fn settings_popup_area(area: Rect) -> Rect {
    centered_rect(area, 68, 12)
}

pub fn config_editor_popup_area(area: Rect) -> Rect {
    centered_rect(area, 92, 18)
}

pub fn file_picker_popup_area(area: Rect) -> Rect {
    centered_rect(area, 78, 22)
}

pub fn file_picker_entries_area(area: Rect) -> Rect {
    file_picker_chunks(file_picker_popup_area(area))[1]
}

fn file_picker_chunks(popup: Rect) -> Rc<[Rect]> {
    let inner = popup.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(inner)
}

#[cfg(test)]
mod tests {
    use super::{centered_rect, edit_display, source_short_label};
    use crate::template::Source;
    use ratatui::prelude::Rect;

    #[test]
    fn source_labels_are_compact() {
        assert_eq!(source_short_label(Source::Global), "g");
        assert_eq!(source_short_label(Source::Local), "l");
    }

    #[test]
    fn edit_display_places_cursor_by_character_index() {
        assert_eq!(edit_display("a中c", 2), "a中▌c");
        assert_eq!(edit_display("a中c", 10), "a中c▌");
    }

    #[test]
    fn centered_rect_stays_inside_area() {
        let area = Rect::new(0, 0, 40, 12);

        let popup = centered_rect(area, 72, 20);

        assert_eq!(popup, Rect::new(1, 1, 38, 10));
    }
}
