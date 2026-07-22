use crate::{
    app::{App, Focus},
    ui,
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::prelude::Rect;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind == KeyEventKind::Release
        || (key.kind == KeyEventKind::Repeat && !repeat_allowed(app, key))
    {
        return;
    }
    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('?') => app.close_help(),
            _ => {}
        }
        return;
    }

    if key.code == KeyCode::F(1) {
        app.toggle_help();
        return;
    }

    if app.show_settings {
        match key.code {
            KeyCode::Esc | KeyCode::F(2) => app.close_settings(),
            KeyCode::Up | KeyCode::Char('k') => app.move_settings(false),
            KeyCode::Down | KeyCode::Char('j') => app.move_settings(true),
            KeyCode::Left if app.settings_idx != 4 => app.adjust_setting(false),
            KeyCode::Right if app.settings_idx != 4 => app.adjust_setting(true),
            KeyCode::Enter | KeyCode::Char(' ') => app.adjust_setting(true),
            _ => {}
        }
        return;
    }

    if key.code == KeyCode::F(2) {
        app.toggle_settings();
        return;
    }

    if app.config_editor.is_some() {
        if app.config_template_property_is_open() {
            if app
                .config_editor
                .as_ref()
                .and_then(|editor| editor.template_property_editor.as_ref())
                .is_some_and(|property_editor| property_editor.editing)
            {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => app.cancel_config_template_property_edit(),
                    (KeyCode::Enter, modifiers)
                        if modifiers.contains(KeyModifiers::CONTROL)
                            || modifiers.contains(KeyModifiers::ALT) =>
                    {
                        app.insert_config_template_property_char('\n');
                    }
                    (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                        app.insert_config_template_property_char('\n');
                    }
                    (KeyCode::Enter, _) => app.commit_config_template_property_edit(),
                    (KeyCode::Backspace, _) => app.backspace_config_template_property_char(),
                    (KeyCode::Delete, _) => app.delete_config_template_property_char(),
                    (KeyCode::Left, _) => app.move_config_template_property_cursor(false),
                    (KeyCode::Right, _) => app.move_config_template_property_cursor(true),
                    (KeyCode::Home, _) => app.move_config_template_property_cursor_to_start(),
                    (KeyCode::End, _) => app.move_config_template_property_cursor_to_end(),
                    (KeyCode::Char(c), _) => app.insert_config_template_property_char(c),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc => app.close_config_template_property_editor(),
                    KeyCode::Up | KeyCode::Char('k') => app.move_config_template_property(false),
                    KeyCode::Down | KeyCode::Char('j') => app.move_config_template_property(true),
                    KeyCode::Enter | KeyCode::Right => app.begin_config_template_property_edit(),
                    KeyCode::Char(' ') => app.toggle_config_template_property(),
                    _ => {}
                }
            }
            return;
        }

        if app
            .config_editor
            .as_ref()
            .is_some_and(|editor| editor.editing)
        {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => app.cancel_config_editor_edit(),
                (KeyCode::Enter, modifiers)
                    if modifiers.contains(KeyModifiers::CONTROL)
                        || modifiers.contains(KeyModifiers::ALT) =>
                {
                    app.insert_config_editor_char('\n');
                }
                (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                    app.insert_config_editor_char('\n');
                }
                (KeyCode::Enter, _) => app.commit_config_editor_edit(),
                (KeyCode::Backspace, _) => app.backspace_config_editor_char(),
                (KeyCode::Delete, _) => app.delete_config_editor_char(),
                (KeyCode::Left, _) => app.move_config_editor_cursor(false),
                (KeyCode::Right, _) => app.move_config_editor_cursor(true),
                (KeyCode::Home, _) => app.move_config_editor_cursor_to_start(),
                (KeyCode::End, _) => app.move_config_editor_cursor_to_end(),
                (KeyCode::Char(c), _) => app.insert_config_editor_char(c),
                _ => {}
            }
        } else {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::F(3), _) => app.close_config_editor(),
                (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                    app.reset_config_editor_to_new_command();
                }
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => app.save_config_editor(),
                (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.move_config_editor(false),
                (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.move_config_editor(true),
                (KeyCode::Enter, _) | (KeyCode::Right, _) => app.begin_config_editor_edit(),
                _ => {}
            }
        }
        return;
    }

    if key.code == KeyCode::F(3) {
        app.open_config_editor();
        return;
    }

    if app.file_picker.is_some() {
        match key.code {
            KeyCode::Esc | KeyCode::Char('f') => app.close_file_picker(),
            KeyCode::Up | KeyCode::Char('k') => app.move_file_picker(false),
            KeyCode::Down | KeyCode::Char('j') => app.move_file_picker(true),
            KeyCode::Left | KeyCode::Backspace => app.file_picker_parent(),
            KeyCode::Right | KeyCode::Enter => app.file_picker_activate(),
            KeyCode::Char(' ') => app.file_picker_select(),
            _ => {}
        }
        return;
    }

    if app.search_editing {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Enter, _) => app.finish_search(),
            (KeyCode::Backspace, _) => app.pop_search_char(),
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => app.reset_current_form_to_defaults(),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => app.clear_search(),
            (KeyCode::Char(c), _) => app.push_search_char(c),
            _ => {}
        }
        return;
    }

    if app.editing {
        if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
            app.reset_current_form_to_defaults();
            return;
        }
        match key.code {
            KeyCode::Esc => app.cancel_edit(),
            KeyCode::Enter => app.commit_edit(),
            KeyCode::Backspace => app.backspace_edit_char(),
            KeyCode::Delete => app.delete_edit_char(),
            KeyCode::Left => app.move_edit_cursor(false),
            KeyCode::Right => app.move_edit_cursor(true),
            KeyCode::Home => app.move_edit_cursor_to_start(),
            KeyCode::End => app.move_edit_cursor_to_end(),
            KeyCode::Char(c) => app.insert_edit_char(c),
            _ => {}
        }
        return;
    }
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => app.should_quit = true,
        (KeyCode::Esc, _) if app.search_active() => app.clear_search(),
        (KeyCode::Char('?'), _) => app.toggle_help(),
        (KeyCode::Char('/'), _) => app.begin_search(),
        (KeyCode::Tab, _) => app.next_focus(false),
        (KeyCode::BackTab, _) => app.next_focus(true),
        (KeyCode::Left, _) => app.next_focus(true),
        (KeyCode::Right, _) => app.next_focus(false),
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.move_sel(false),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.move_sel(true),
        (KeyCode::Enter, _) => app.activate(),
        (KeyCode::Char('f'), KeyModifiers::NONE) => app.open_file_picker(),
        (KeyCode::Char(' '), _) => app.toggle(),
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => app.reset_current_form_to_defaults(),
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => app.reload(),
        (KeyCode::Char('y'), KeyModifiers::CONTROL) => app.confirm(),
        _ => {}
    }
}

fn repeat_allowed(app: &App, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return false;
    }
    match key.code {
        KeyCode::Up
        | KeyCode::Down
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Backspace
        | KeyCode::Delete => true,
        KeyCode::Char('j' | 'k') => true,
        KeyCode::Char(_) => {
            app.editing
                || app.search_editing
                || app.config_editor.as_ref().is_some_and(|editor| {
                    editor.editing
                        || editor
                            .template_property_editor
                            .as_ref()
                            .is_some_and(|property| property.editing)
                })
        }
        _ => false,
    }
}

pub fn handle_paste(app: &mut App, text: &str) {
    if app.config_template_property_is_open()
        && app
            .config_editor
            .as_ref()
            .and_then(|editor| editor.template_property_editor.as_ref())
            .is_some_and(|property_editor| property_editor.editing)
    {
        app.insert_config_template_property_text(text);
    } else if app
        .config_editor
        .as_ref()
        .is_some_and(|editor| editor.editing)
    {
        app.insert_config_editor_text(text);
    } else if app.editing {
        for ch in text.chars() {
            app.insert_edit_char(ch);
        }
    } else if app.search_editing {
        for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
            app.push_search_char(ch);
        }
    }
}

pub fn handle_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
    if app.show_help {
        return;
    }
    if app.show_settings {
        handle_settings_mouse(app, mouse, screen);
        return;
    }
    if app.config_editor.is_some() {
        handle_config_editor_mouse(app, mouse, screen);
        return;
    }
    if app.file_picker.is_some() {
        handle_file_picker_mouse(app, mouse, screen);
        return;
    }

    let areas = ui::areas(screen);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if contains(areas.execute_button, mouse.column, mouse.row) {
                app.confirm();
            } else if let Some(idx) = item_index(areas.categories, mouse.column, mouse.row) {
                app.select_category(app.category_offset + idx);
            } else if let Some(idx) = item_index(areas.commands, mouse.column, mouse.row) {
                app.select_command(app.command_offset + idx);
            } else if let Some(idx) = item_index(areas.form, mouse.column, mouse.row) {
                app.select_form_item(app.form_offset + idx, true);
            }
        }
        MouseEventKind::ScrollUp => scroll_at(app, mouse.column, mouse.row, screen, false),
        MouseEventKind::ScrollDown => scroll_at(app, mouse.column, mouse.row, screen, true),
        _ => {}
    }
}

fn handle_settings_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
    let popup = ui::settings_popup_area(screen);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = item_index(popup, mouse.column, mouse.row) {
                app.select_setting(app.settings_offset + idx, true);
            }
        }
        MouseEventKind::ScrollUp => app.move_settings(false),
        MouseEventKind::ScrollDown => app.move_settings(true),
        _ => {}
    }
}

fn handle_config_editor_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
    if app.config_template_property_is_open() {
        handle_config_template_property_mouse(app, mouse, screen);
        return;
    }
    let popup = ui::config_editor_popup_area(screen);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = item_index(popup, mouse.column, mouse.row) {
                let offset = app.config_editor.as_ref().map_or(0, |editor| editor.offset);
                app.select_config_editor_field(offset + idx, true);
            }
        }
        MouseEventKind::ScrollUp
            if contains(popup, mouse.column, mouse.row) && !config_editor_is_editing(app) =>
        {
            app.move_config_editor(false);
        }
        MouseEventKind::ScrollDown
            if contains(popup, mouse.column, mouse.row) && !config_editor_is_editing(app) =>
        {
            app.move_config_editor(true);
        }
        _ => {}
    }
}

fn handle_config_template_property_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
    let popup = ui::config_template_property_popup_area(screen);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = item_index(popup, mouse.column, mouse.row) {
                let offset = app
                    .config_editor
                    .as_ref()
                    .and_then(|editor| editor.template_property_editor.as_ref())
                    .map_or(0, |property| property.offset);
                app.select_config_template_property(offset + idx, true);
            }
        }
        MouseEventKind::ScrollUp
            if contains(popup, mouse.column, mouse.row)
                && !config_template_property_is_editing(app) =>
        {
            app.move_config_template_property(false);
        }
        MouseEventKind::ScrollDown
            if contains(popup, mouse.column, mouse.row)
                && !config_template_property_is_editing(app) =>
        {
            app.move_config_template_property(true);
        }
        _ => {}
    }
}

fn config_editor_is_editing(app: &App) -> bool {
    app.config_editor
        .as_ref()
        .is_some_and(|editor| editor.editing)
}

fn config_template_property_is_editing(app: &App) -> bool {
    app.config_editor
        .as_ref()
        .and_then(|editor| editor.template_property_editor.as_ref())
        .is_some_and(|property_editor| property_editor.editing)
}

fn handle_file_picker_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
    let entries = ui::file_picker_entries_area(screen);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = plain_item_index(entries, mouse.column, mouse.row) {
                let offset = app.file_picker.as_ref().map_or(0, |picker| picker.offset);
                app.select_file_picker_entry(offset + idx, true);
            }
        }
        MouseEventKind::ScrollUp => app.move_file_picker(false),
        MouseEventKind::ScrollDown => app.move_file_picker(true),
        _ => {}
    }
}

fn scroll_at(app: &mut App, column: u16, row: u16, screen: Rect, down: bool) {
    let areas = ui::areas(screen);
    if contains(areas.categories, column, row) {
        app.focus = Focus::Categories;
        app.move_sel(down);
    } else if contains(areas.commands, column, row) {
        app.focus = Focus::Commands;
        app.move_sel(down);
    } else if contains(areas.form, column, row) {
        app.focus = Focus::Form;
        app.move_sel(down);
    } else if contains(areas.preview, column, row) {
        let (width, height) = ui::preview_viewport_size(areas.preview);
        app.scroll_preview(down, width, height);
    }
}

fn item_index(area: Rect, column: u16, row: u16) -> Option<usize> {
    if contains(area, column, row)
        && row > area.y
        && row < area.y.saturating_add(area.height).saturating_sub(1)
    {
        Some((row - area.y - 1) as usize)
    } else {
        None
    }
}

fn plain_item_index(area: Rect, column: u16, row: u16) -> Option<usize> {
    if contains(area, column, row) {
        Some((row - area.y) as usize)
    } else {
        None
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{FilePicker, FilePickerEntry},
        template::{Category, Command, Config, Settings, Source},
    };
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    use indexmap::IndexMap;
    use std::path::PathBuf;

    #[test]
    fn mouse_click_starts_config_editor_field_editing() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        let screen = Rect::new(0, 0, 100, 30);
        let popup = ui::config_editor_popup_area(screen);

        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: popup.x + 2,
                row: popup.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );

        let editor = app.config_editor.as_ref().unwrap();
        assert_eq!(editor.selected, 0);
        assert!(editor.editing);
    }

    #[test]
    fn mouse_wheel_does_not_move_config_editor_selection_while_editing() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        let screen = Rect::new(0, 0, 100, 30);
        let popup = ui::config_editor_popup_area(screen);

        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: popup.x + 2,
                row: popup.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );
        let before = app.config_editor.as_ref().unwrap().edit_buffer.clone();

        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: popup.x + 2,
                row: popup.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );

        let editor = app.config_editor.as_ref().unwrap();
        assert_eq!(editor.selected, 0);
        assert!(editor.editing);
        assert_eq!(editor.edit_buffer, before);
    }

    #[test]
    fn mouse_click_opens_config_template_property_editor() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        let screen = Rect::new(0, 0, 100, 30);
        let popup = ui::config_editor_popup_area(screen);

        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: popup.x + 2,
                row: popup.y + 1 + 9,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );

        let editor = app.config_editor.as_ref().unwrap();
        assert_eq!(editor.selected, 9);
        assert_eq!(
            editor
                .template_property_editor
                .as_ref()
                .map(|property_editor| property_editor.part_index),
            Some(0)
        );
    }

    #[test]
    fn mouse_wheel_does_not_move_template_property_selection_while_editing() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        app.open_config_template_property_editor(0);
        app.begin_config_template_property_edit();
        let screen = Rect::new(0, 0, 100, 30);
        let popup = ui::config_template_property_popup_area(screen);
        let before = app
            .config_editor
            .as_ref()
            .unwrap()
            .template_property_editor
            .as_ref()
            .unwrap()
            .edit_buffer
            .clone();

        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: popup.x + 2,
                row: popup.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );

        let property_editor = app
            .config_editor
            .as_ref()
            .unwrap()
            .template_property_editor
            .as_ref()
            .unwrap();
        assert_eq!(property_editor.selected, 0);
        assert!(property_editor.editing);
        assert_eq!(property_editor.edit_buffer, before);
    }

    #[test]
    fn mouse_wheel_scrolls_preview_when_over_preview_area() {
        let mut app = App::new(long_preview_config());
        let screen = Rect::new(0, 0, 40, 24);
        let preview = ui::areas(screen).preview;

        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: preview.x + 2,
                row: preview.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );

        assert_eq!(app.preview_scroll, 1);
        assert_eq!(app.focus, Focus::Categories);
    }

    fn long_preview_config() -> Config {
        let mut categories = IndexMap::new();
        categories.insert(
            "dev".to_string(),
            Category {
                alias: Some("开发".to_string()),
                source: Source::Global,
            },
        );
        let mut commands = IndexMap::new();
        commands.insert(
            "long".to_string(),
            Command {
                category: "dev".to_string(),
                title: Some("长命令".to_string()),
                description: None,
                danger: false,
                template: format!("echo {}", "1234567890 ".repeat(32)),
                params: vec![],
                options: vec![],
                source: Source::Global,
            },
        );
        Config {
            settings: Settings::default(),
            categories,
            commands,
            sources: vec![],
        }
    }

    #[test]
    fn paste_inserts_multiline_text_in_config_editor_field() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        app.select_config_editor_field(6, true);
        {
            let editor = app.config_editor.as_mut().unwrap();
            editor.edit_buffer.clear();
            editor.edit_cursor = 0;
        }

        handle_paste(&mut app, "echo one\necho two");
        app.commit_config_editor_edit();

        assert_eq!(
            app.config_editor.as_ref().unwrap().draft.template,
            "echo one\necho two"
        );
    }

    #[test]
    fn config_editor_ctrl_j_inserts_newline() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        app.select_config_editor_field(6, true);
        {
            let editor = app.config_editor.as_mut().unwrap();
            editor.edit_buffer.clear();
            editor.edit_cursor = 0;
        }
        app.insert_config_editor_text("echo one");

        handle_key(
            &mut app,
            KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
        );
        app.insert_config_editor_text("echo two");
        app.commit_config_editor_edit();

        assert_eq!(
            app.config_editor.as_ref().unwrap().draft.template,
            "echo one\necho two"
        );
    }

    #[test]
    fn mouse_click_selects_file_picker_entry() {
        let mut app = App::new(Config::default());
        app.file_picker = Some(FilePicker {
            param_name: "path".to_string(),
            dir: PathBuf::from("."),
            entries: vec![FilePickerEntry {
                name: "Cargo.toml".to_string(),
                path: PathBuf::from("Cargo.toml"),
                is_dir: false,
            }],
            selected: 0,
            offset: 0,
            error: None,
        });
        let screen = Rect::new(0, 0, 100, 30);
        let entries = ui::file_picker_entries_area(screen);

        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: entries.x,
                row: entries.y,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );

        assert_eq!(
            app.values.get("path").map(String::as_str),
            Some("Cargo.toml")
        );
        assert!(app.file_picker.is_none());
    }

    #[test]
    fn repeat_and_release_do_not_confirm_a_dangerous_command_twice() {
        let mut config = long_preview_config();
        config.commands.get_mut("long").unwrap().danger = true;
        let mut app = App::new(config);
        let key = |kind| KeyEvent {
            code: KeyCode::Char('y'),
            modifiers: KeyModifiers::CONTROL,
            kind,
            state: KeyEventState::NONE,
        };
        handle_key(&mut app, key(KeyEventKind::Press));
        assert!(app.danger_confirmation.is_some());
        assert!(app.output.is_none());
        handle_key(&mut app, key(KeyEventKind::Repeat));
        handle_key(&mut app, key(KeyEventKind::Release));
        assert!(app.danger_confirmation.is_some());
        assert!(app.output.is_none());
        assert!(!app.should_quit);
    }

    #[test]
    fn mouse_click_uses_persisted_list_offset() {
        let mut config = Config::default();
        for index in 0..20 {
            config.categories.insert(
                format!("cat_{index}"),
                Category {
                    alias: None,
                    source: Source::Global,
                },
            );
        }
        let mut app = App::new(config);
        app.category_idx = 12;
        app.category_offset = 8;
        let screen = Rect::new(0, 0, 80, 12);
        let area = ui::areas(screen).categories;
        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: area.x + 1,
                row: area.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );
        assert_eq!(app.category_idx, 8);
    }
}
