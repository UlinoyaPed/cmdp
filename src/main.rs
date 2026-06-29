mod app;
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
mod ui;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, poll, read},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, process, time::Duration};

fn main() -> Result<()> {
    let cfg = config::load()?;
    let mut app = app::App::new(cfg);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = run(&mut terminal, &mut app);
    app.persist_exit_state();
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res?;
    if let Some(cmd) = app.output {
        let status = output::execute_command(&cmd)?;
        process::exit(status.code().unwrap_or(1));
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
