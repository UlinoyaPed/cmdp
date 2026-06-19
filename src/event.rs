use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.move_sel(false),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.move_sel(true),
        (KeyCode::Enter, _) => app.activate(),
        (KeyCode::Char(' '), _) => app.toggle(),
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => app.reload(),
        (KeyCode::Char('y'), KeyModifiers::CONTROL) => app.confirm(),
        _ => {}
    }
}
