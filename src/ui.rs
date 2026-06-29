use crate::{
    app::{App, ConfigEditTarget, Focus, FormItem, TemplatePart, TemplatePartKind},
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
    if app.config_template_property_is_open() {
        draw_config_template_property_popup(f, app, f.area());
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
    let title = format!(
        "{}  {} ",
        texts.config_editor_title,
        config_editor_target(editor, texts)
    );
    let mut rows = vec![
        config_editor_item(
            texts.config_editor_command_id,
            editor_value(app, editor, 0),
            editor.selected == 0 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_category_id,
            editor_value(app, editor, 1),
            editor.selected == 1 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_category_alias,
            editor_value(app, editor, 2),
            editor.selected == 2 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_command_title,
            editor_value(app, editor, 3),
            editor.selected == 3 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_description,
            editor_value(app, editor, 4),
            editor.selected == 4 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_danger,
            editor_value(app, editor, 5),
            editor.selected == 5 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_template,
            editor_value(app, editor, 6),
            editor.selected == 6 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_params,
            editor_value(app, editor, 7),
            editor.selected == 7 && editor.editing,
        ),
        config_editor_item(
            texts.config_editor_options,
            editor_value(app, editor, 8),
            editor.selected == 8 && editor.editing,
        ),
    ];
    let labels = app.config_editor_template_part_labels();
    for (idx, part) in app.config_editor_template_parts().iter().enumerate() {
        rows.push(config_template_part_item(
            texts,
            part,
            labels.get(idx).map(String::as_str).unwrap_or_default(),
            editor.selected == 9 + idx,
        ));
    }
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
                        title,
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

fn config_editor_target(editor: &crate::app::ConfigEditor, texts: &Texts) -> String {
    match &editor.target {
        ConfigEditTarget::GlobalEditor => texts.config_editor_target_global.to_string(),
        ConfigEditTarget::LocalProject(path) => format!(
            "{}{}",
            texts.config_editor_target_local_prefix,
            truncate(&path.display().to_string(), 42)
        ),
    }
}

fn editor_value(app: &App, editor: &crate::app::ConfigEditor, idx: usize) -> String {
    if editor.selected == idx && editor.editing {
        edit_display(&editor.edit_buffer, editor.edit_cursor)
    } else {
        match idx {
            0 => truncate(
                &id_with_alias(&editor.draft.command_id, &editor.draft.title),
                56,
            ),
            1 => truncate(
                &id_with_alias(&editor.draft.category_id, &editor.draft.category_alias),
                56,
            ),
            7 | 8 => app
                .config_editor_field_preview(idx)
                .map(|value| truncate(&compact_newlines(&value), 56))
                .unwrap_or_else(|| truncate(&compact_newlines(editor.draft.field(idx)), 56)),
            _ => truncate(&compact_newlines(editor.draft.field(idx)), 56),
        }
    }
}

fn id_with_alias(id: &str, alias: &str) -> String {
    let alias = alias.trim();
    if alias.is_empty() || alias == id {
        id.to_string()
    } else {
        format!("{id}  {alias}")
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

fn config_template_part_item(
    texts: &Texts,
    part: &TemplatePart,
    label: &str,
    selected: bool,
) -> ListItem<'static> {
    let value_style = if selected {
        Style::default().fg(Color::Black).bg(Color::LightCyan)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let kind = match &part.kind {
        TemplatePartKind::Required => texts.config_template_required_part,
        TemplatePartKind::Optional { .. } => texts.config_template_optional_part,
    };
    let detail = label.to_string();
    ListItem::new(Line::from(vec![
        Span::styled(kind.to_string(), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(truncate(&compact_newlines(&part.token), 34), value_style),
        Span::raw("  "),
        Span::styled(detail, Style::default().fg(Color::DarkGray)),
    ]))
}

fn draw_config_template_property_popup(f: &mut Frame, app: &App, area: Rect) {
    let Some(editor) = &app.config_editor else {
        return;
    };
    let Some(property_editor) = &editor.template_property_editor else {
        return;
    };
    let texts = app.texts();
    let popup = config_template_property_popup_area(area);
    let fields = app.config_template_property_fields();
    let title = format!(
        "{} {} ",
        texts.config_template_property_title,
        app.config_template_property_part_label()
            .map(|label| truncate(&label, 48))
            .unwrap_or_default()
    );
    let rows = if fields.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            texts.config_template_property_empty,
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        fields
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let value = if property_editor.selected == idx && property_editor.editing {
                    edit_display(&property_editor.edit_buffer, property_editor.edit_cursor)
                } else {
                    truncate(&compact_newlines(&field.value), 46)
                };
                config_editor_item(
                    &field.label,
                    value,
                    property_editor.selected == idx && property_editor.editing,
                )
            })
            .collect()
    };
    let mut state = ListState::default();
    state.select(Some(
        property_editor.selected.min(rows.len().saturating_sub(1)),
    ));

    f.render_widget(Clear, popup);
    f.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .block(
                Block::default()
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::LightCyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .title_bottom(Span::styled(
                        texts.config_template_property_help,
                        Style::default().fg(Color::DarkGray),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::LightCyan)),
            ),
        popup,
        &mut state,
    );
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

fn form_block(title: &str, focus: bool, help: Option<String>) -> Block<'static> {
    let block = block(title, focus);
    if let Some(help) = help {
        block.title_bottom(Span::styled(help, Style::default().fg(Color::LightBlue)))
    } else {
        block
    }
}

fn command_block(title: &str, focus: bool, help: Option<String>) -> Block<'static> {
    let block = block(title, focus);
    if let Some(help) = help {
        block.title_bottom(Span::styled(help, Style::default().fg(Color::LightBlue)))
    } else {
        block
    }
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
    let command_help = command_help_text(texts, &commands, app.command_idx, area.width);
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
            .block(command_block(
                &title,
                app.focus == Focus::Commands || app.search_editing,
                command_help,
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

fn command_description(command: &crate::template::Command) -> Option<&str> {
    command
        .description
        .as_deref()
        .map(str::trim)
        .filter(|description| !description.is_empty())
}

fn command_help_text(
    texts: &Texts,
    commands: &[(&String, &crate::template::Command)],
    selected: usize,
    area_width: u16,
) -> Option<String> {
    let description = commands
        .get(selected)
        .and_then(|(_, command)| command_description(command))?;
    let max_chars = area_width.saturating_sub(4).max(8) as usize;
    Some(truncate(
        &format!("{}{}", texts.command_help_prefix, description),
        max_chars,
    ))
}

fn draw_form(f: &mut Frame, app: &App, area: Rect) {
    let texts = app.texts();
    let form_items = app.form_items();
    let form_help = if app.focus == Focus::Form {
        form_help_text(texts, &form_items, app.form_idx, area.width)
    } else {
        None
    };
    let mut rows: Vec<ListItem> = Vec::new();

    for (idx, item) in form_items.into_iter().enumerate() {
        let selected = app.focus == Focus::Form && idx == app.form_idx;
        rows.push(match item {
            FormItem::Param {
                label,
                value,
                placeholder,
                help: _,
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
            .block(form_block(
                texts.form_title,
                app.focus == Focus::Form,
                form_help,
            )),
        area,
        &mut state,
    );
}

fn form_help_text(
    texts: &Texts,
    items: &[FormItem],
    selected: usize,
    area_width: u16,
) -> Option<String> {
    let help = match items.get(selected) {
        Some(FormItem::Param {
            help: Some(help), ..
        }) => help.trim(),
        _ => "",
    };
    if help.is_empty() {
        return None;
    }
    let max_chars = area_width.saturating_sub(4).max(8) as usize;
    Some(truncate(
        &format!("{}{}", texts.form_help_prefix, help),
        max_chars,
    ))
}

#[allow(clippy::too_many_arguments)]
fn param_item(
    texts: &Texts,
    app: &App,
    selected: bool,
    label: String,
    value: String,
    placeholder: Option<String>,
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
    let detail = placeholder
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
            display.push('█');
        }
        display.push(ch);
    }
    if cursor == value.chars().count() {
        display.push('█');
    }
    compact_newlines(&display)
}

fn compact_newlines(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str(" ↵ ");
            }
            '\n' => out.push_str(" ↵ "),
            _ => out.push(ch),
        }
    }
    out
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

pub fn config_template_property_popup_area(area: Rect) -> Rect {
    centered_rect(area, 78, 16)
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
    use super::{
        centered_rect, command_description, command_help_text, edit_display, form_help_text,
        source_short_label,
    };
    use crate::app::FormItem;
    use crate::i18n::ZH_CN;
    use crate::template::{Command, Source};
    use ratatui::prelude::Rect;

    #[test]
    fn source_labels_are_compact() {
        assert_eq!(source_short_label(Source::Global), "g");
        assert_eq!(source_short_label(Source::Local), "l");
    }

    #[test]
    fn edit_display_places_cursor_by_character_index() {
        assert_eq!(edit_display("a中c", 2), "a中█c");
        assert_eq!(edit_display("a中c", 10), "a中c█");
    }

    #[test]
    fn edit_display_compacts_newlines_for_list_rows() {
        assert_eq!(
            edit_display("echo one\necho two", 8),
            "echo one█ ↵ echo two"
        );
    }

    #[test]
    fn form_help_text_uses_selected_parameter_help() {
        let items = vec![FormItem::Param {
            name: "mode".to_string(),
            label: "模式".to_string(),
            value: String::new(),
            placeholder: None,
            help: Some("soft 保留修改，hard 会丢弃工作区修改".to_string()),
            secret: false,
            choices: vec![],
            required: true,
        }];

        assert_eq!(
            form_help_text(&ZH_CN, &items, 0, 80).as_deref(),
            Some("帮助：soft 保留修改，hard 会丢弃工作区修改")
        );
    }

    #[test]
    fn form_help_text_ignores_options_and_empty_help() {
        let items = vec![FormItem::Option {
            id: "verbose".to_string(),
            label: "详细输出".to_string(),
            enabled: false,
        }];

        assert!(form_help_text(&ZH_CN, &items, 0, 80).is_none());
    }

    #[test]
    fn command_description_trims_and_ignores_empty_text() {
        let mut command = test_command();

        command.description = Some("  查看当前仓库状态  ".to_string());
        assert_eq!(command_description(&command), Some("查看当前仓库状态"));

        command.description = Some("   ".to_string());
        assert_eq!(command_description(&command), None);
    }

    #[test]
    fn command_help_text_uses_selected_command_description() {
        let id = "git_status".to_string();
        let mut command = test_command();
        command.description = Some("查看当前仓库状态".to_string());
        let commands = vec![(&id, &command)];

        assert_eq!(
            command_help_text(&ZH_CN, &commands, 0, 80).as_deref(),
            Some("说明：查看当前仓库状态")
        );
    }

    #[test]
    fn centered_rect_stays_inside_area() {
        let area = Rect::new(0, 0, 40, 12);

        let popup = centered_rect(area, 72, 20);

        assert_eq!(popup, Rect::new(1, 1, 38, 10));
    }

    fn test_command() -> Command {
        Command {
            category: "git".to_string(),
            title: Some("Git 状态".to_string()),
            description: None,
            danger: false,
            template: "git status".to_string(),
            params: vec![],
            options: vec![],
            source: Source::Global,
        }
    }
}
