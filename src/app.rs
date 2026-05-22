use std::time::Duration;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    config::AppConfig,
    db::Database,
    notifications::Notifier,
    timer::{Phase, Timer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Timer,
    Statistics,
}

pub struct App {
    config: AppConfig,
    timer: Timer,
    database: Database,
    notifier: Notifier,
    active_tab: ActiveTab,
    should_quit: bool,
}

impl App {
    pub fn new(config: AppConfig, database: Database, notifier: Notifier) -> Self {
        let timer = Timer::new(&config.timers);
        Self {
            config,
            timer,
            database,
            notifier,
            active_tab: ActiveTab::Timer,
            should_quit: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('t') => self.toggle_tab(),
            KeyCode::Char(' ') => self.timer.start_pause(),
            KeyCode::Char('s') => self.timer.stop(&self.config.timers),
            KeyCode::Char('r') => self.timer.reset(&self.config.timers),
            KeyCode::Char('n') => self.timer.next(&self.config.timers),
            _ => {}
        }
    }

    pub fn tick(&mut self, elapsed: Duration) -> Result<()> {
        if let Some(iteration) = self.timer.tick(elapsed, &self.config.timers) {
            self.database.insert_iteration(&iteration)?;

            // Notification delivery can fail in headless WSL sessions; keep the timer usable.
            let _ = self.notifier.notify_completed(&iteration);
        }
        Ok(())
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn active_tab(&self) -> ActiveTab {
        self.active_tab
    }

    pub fn active_tab_index(&self) -> usize {
        match self.active_tab {
            ActiveTab::Timer => 0,
            ActiveTab::Statistics => 1,
        }
    }

    pub fn phase(&self) -> Phase {
        self.timer.phase()
    }

    pub fn is_running(&self) -> bool {
        self.timer.is_running()
    }

    pub fn clock_text(&self) -> String {
        let secs = self.timer.remaining().as_secs();
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }

    fn toggle_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Timer => ActiveTab::Statistics,
            ActiveTab::Statistics => ActiveTab::Timer,
        };
    }
}
