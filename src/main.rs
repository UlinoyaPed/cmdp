mod app;
mod atomic;
mod config;
mod error;
mod event;
mod i18n;
mod output;
mod parser;
mod preview;
mod renderer;
mod state;
mod template;
mod terminal_session;
mod ui;
use anyhow::Result;
use crossterm::event::{Event, poll, read};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, process, time::Duration};

fn main() -> Result<()> {
    let cfg = config::load()?;
    let mut app = app::App::new(cfg);
    let mut session = terminal_session::TerminalSession::enter()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = run(&mut terminal, &mut app);
    app.persist_exit_state();
    drop(terminal);
    let restore = session.restore();
    res?;
    restore?;
    if let Some(cmd) = app.output {
        let status = output::execute_command(&cmd)?;
        process::exit(output::exit_code(status));
    }
    Ok(())
}
fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut app::App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, app))?;
        if poll(Duration::from_millis(200))? {
            match read()? {
                Event::Key(key) => event::handle_key(app, key),
                Event::Mouse(mouse) => event::handle_mouse(app, mouse, terminal.size()?.into()),
                Event::Paste(text) => event::handle_paste(app, &text),
                Event::Resize(_, _) | Event::FocusGained | Event::FocusLost => {}
            }
        }
    }
    Ok(())
}
