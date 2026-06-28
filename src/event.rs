use crate::{
    app::{App, Focus},
    ui,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::prelude::Rect;

pub fn handle_key(app: &mut App, key: KeyEvent) {
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
            KeyCode::Left => app.adjust_setting(false),
            KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => app.adjust_setting(true),
            _ => {}
        }
        return;
    }

    if key.code == KeyCode::F(2) {
        app.toggle_settings();
        return;
    }

    if app.config_editor.is_some() {
        if app
            .config_editor
            .as_ref()
            .is_some_and(|editor| editor.editing)
        {
            match key.code {
                KeyCode::Esc => app.cancel_config_editor_edit(),
                KeyCode::Enter => app.commit_config_editor_edit(),
                KeyCode::Backspace => app.backspace_config_editor_char(),
                KeyCode::Delete => app.delete_config_editor_char(),
                KeyCode::Left => app.move_config_editor_cursor(false),
                KeyCode::Right => app.move_config_editor_cursor(true),
                KeyCode::Home => app.move_config_editor_cursor_to_start(),
                KeyCode::End => app.move_config_editor_cursor_to_end(),
                KeyCode::Char(c) => app.insert_config_editor_char(c),
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
                app.select_category(idx);
            } else if let Some(idx) = item_index(areas.commands, mouse.column, mouse.row) {
                app.select_command(idx);
            } else if let Some(idx) = item_index(areas.form, mouse.column, mouse.row) {
                app.select_form_item(idx, true);
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
                app.select_setting(idx, true);
            }
        }
        MouseEventKind::ScrollUp => app.move_settings(false),
        MouseEventKind::ScrollDown => app.move_settings(true),
        _ => {}
    }
}

fn handle_config_editor_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
    let popup = ui::config_editor_popup_area(screen);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = item_index(popup, mouse.column, mouse.row) {
                app.select_config_editor_field(idx, true);
            }
        }
        MouseEventKind::ScrollUp => app.move_config_editor(false),
        MouseEventKind::ScrollDown => app.move_config_editor(true),
        _ => {}
    }
}

fn handle_file_picker_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
    let entries = ui::file_picker_entries_area(screen);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = plain_item_index(entries, mouse.column, mouse.row) {
                app.select_file_picker_entry(idx, true);
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
        template::Config,
    };
    use crossterm::event::KeyModifiers;
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
}
