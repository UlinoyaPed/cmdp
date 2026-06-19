use crate::{
    app::{App, Focus},
    ui,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::prelude::Rect;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.search_editing {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Enter, _) => app.finish_search(),
            (KeyCode::Backspace, _) => app.pop_search_char(),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => app.clear_search(),
            (KeyCode::Char(c), _) => app.push_search_char(c),
            _ => {}
        }
        return;
    }

    if app.editing {
        match key.code {
            KeyCode::Esc => app.editing = false,
            KeyCode::Enter => app.commit_edit(),
            KeyCode::Backspace => {
                app.edit_buffer.pop();
            }
            KeyCode::Char(c) => app.edit_buffer.push(c),
            _ => {}
        }
        return;
    }
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => app.should_quit = true,
        (KeyCode::Esc, _) if app.search_active() => app.clear_search(),
        (KeyCode::Char('/'), _) => app.begin_search(),
        (KeyCode::Tab, _) => app.next_focus(false),
        (KeyCode::BackTab, _) => app.next_focus(true),
        (KeyCode::Left, _) => app.next_focus(true),
        (KeyCode::Right, _) => app.next_focus(false),
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.move_sel(false),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.move_sel(true),
        (KeyCode::Enter, _) => app.activate(),
        (KeyCode::Char(' '), _) => app.toggle(),
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => app.reload(),
        (KeyCode::Char('y'), KeyModifiers::CONTROL) => app.confirm(),
        _ => {}
    }
}

pub fn handle_mouse(app: &mut App, mouse: MouseEvent, screen: Rect) {
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

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}
