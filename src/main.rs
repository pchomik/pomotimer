use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod config;
mod db;
mod notifications;
mod timer;
mod ui;

use app::App;
use db::Database;
use notifications::Notifier;

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    let config = config::load_or_create()?;
    let database = Database::open()?;
    let notifier = Notifier::new(config.notifications.clone());
    let mut app = App::new(config, database, notifier);

    let mut terminal = init_terminal()?;
    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;

    result
}

fn run(terminal: &mut Tui, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        let now = Instant::now();
        app.tick(now.saturating_duration_since(last_tick))?;
        last_tick = now;

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

fn init_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
