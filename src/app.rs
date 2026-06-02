use std::time::Duration;

use anyhow::Result;
use chrono::{Datelike, Local, Months, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    config::AppConfig,
    db::{DailyAgg, Database, MonthlyAgg},
    notifications::Notifier,
    timer::{Phase, Timer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Timer,
    Statistics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsScale {
    Week,
    Month,
}

pub struct App {
    config: AppConfig,
    timer: Timer,
    database: Database,
    notifier: Notifier,
    active_tab: ActiveTab,
    should_quit: bool,
    stats_scale: StatsScale,
    daily_offset: i32,
    monthly_offset: i32,
    daily_data: Vec<DailyAgg>,
    monthly_data: Vec<MonthlyAgg>,
    stats_bar_count: u32,
}

impl App {
    pub fn new(config: AppConfig, database: Database, notifier: Notifier) -> Self {
        let timer = Timer::new(&config.timers);

        let mut app = Self {
            config,
            timer,
            database,
            notifier,
            active_tab: ActiveTab::Timer,
            should_quit: false,
            stats_scale: StatsScale::Week,
            daily_offset: 0,
            monthly_offset: 0,
            daily_data: Vec::new(),
            monthly_data: Vec::new(),
            stats_bar_count: 7,
        };

        // Pre-load stats so the Statistics tab has data immediately.
        let _ = app.refresh_stats();
        app
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.active_tab() {
            ActiveTab::Timer => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('t') => self.toggle_tab(),
                KeyCode::Char(' ') => self.timer.start_pause(),
                KeyCode::Char('s') => self.timer.stop(&self.config.timers),
                KeyCode::Char('r') => self.timer.reset(&self.config.timers),
                KeyCode::Char('n') => self
                    .timer
                    .next(&self.config.timers, self.config.timers.auto_start),
                _ => {}
            },
            ActiveTab::Statistics => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('t') => {
                    self.toggle_tab();
                    let _ = self.refresh_stats();
                }
                KeyCode::Char('s') => {
                    self.stats_scale = match self.stats_scale {
                        StatsScale::Month => StatsScale::Week,
                        StatsScale::Week => StatsScale::Month,
                    };
                    let _ = self.refresh_stats();
                }
                KeyCode::Char('n') => {
                    match self.stats_scale {
                        StatsScale::Week => {
                            self.daily_offset = (self.daily_offset + 1).min(0);
                        }
                        StatsScale::Month => {
                            self.monthly_offset = (self.monthly_offset + 1).min(0);
                        }
                    }
                    let _ = self.refresh_stats();
                }
                KeyCode::Char('p') => {
                    match self.stats_scale {
                        StatsScale::Week => {
                            self.daily_offset -= 1;
                        }
                        StatsScale::Month => {
                            self.monthly_offset -= 1;
                        }
                    }
                    let _ = self.refresh_stats();
                }
                KeyCode::Char('r') => {
                    self.daily_offset = 0;
                    self.monthly_offset = 0;
                    self.stats_scale = StatsScale::Week;
                    let _ = self.refresh_stats();
                }
                _ => {}
            },
        }
    }

    pub fn tick(&mut self, elapsed: Duration) -> Result<()> {
        if let Some(iteration) = self.timer.tick(elapsed, &self.config.timers) {
            self.database.insert_iteration(&iteration)?;

            // Notification delivery can fail in headless WSL sessions; keep the timer usable.
            let _ = self.notifier.notify_completed(&iteration);

            // Auto-refresh stats if user is viewing them when a pomodoro finishes.
            if self.active_tab == ActiveTab::Statistics {
                let _ = self.refresh_stats();
            }
        }
        Ok(())
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn active_tab(&self) -> ActiveTab {
        self.active_tab
    }

    pub fn stats_scale(&self) -> StatsScale {
        self.stats_scale
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

    // --- Statistics accessors ---

    pub fn daily_data(&self) -> &[DailyAgg] {
        &self.daily_data
    }

    pub fn monthly_data(&self) -> &[MonthlyAgg] {
        &self.monthly_data
    }

    pub fn set_stats_bar_count(&mut self, count: u32) {
        let count = count.max(1);
        if count != self.stats_bar_count {
            self.stats_bar_count = count;
            let _ = self.refresh_stats();
        }
    }

    pub fn daily_window_range(&self, count: u32) -> (NaiveDate, NaiveDate) {
        let today = Local::now().date_naive();
        let offset_days = self.daily_offset * count as i32;
        let count = count.max(1);
        let end = if offset_days >= 0 {
            today
                .checked_add_days(chrono::Days::new(offset_days as u64))
                .unwrap_or(today)
                .min(today)
        } else {
            today
                .checked_sub_days(chrono::Days::new((-offset_days) as u64))
                .unwrap_or(today)
        };
        let start = end
            .checked_sub_days(chrono::Days::new((count - 1) as u64))
            .unwrap_or(end);
        (start, end)
    }

    pub fn monthly_window_range(&self, count: u32) -> (NaiveDate, NaiveDate) {
        let today = Local::now().date_naive();
        let current_month_first =
            NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);

        let count = count.max(1);
        let end_month_first = if self.monthly_offset <= 0 {
            current_month_first
                .checked_sub_months(Months::new((-self.monthly_offset) as u32))
                .unwrap_or(current_month_first)
        } else {
            // offset > 0 clamped to 0, shouldn't happen but handle gracefully.
            current_month_first
        };

        let start_month_first = end_month_first
            .checked_sub_months(Months::new(count - 1))
            .unwrap_or(end_month_first);

        (start_month_first, end_month_first)
    }

    fn refresh_stats(&mut self) -> Result<()> {
        match self.stats_scale {
            StatsScale::Week => {
                let (start, end) = self.daily_window_range(self.stats_bar_count);
                self.daily_data = self.database.daily_window(start, end)?;
            }
            StatsScale::Month => {
                let (start, end) = self.monthly_window_range(self.stats_bar_count);
                self.monthly_data = self.database.monthly_window(start, end)?;
            }
        }
        Ok(())
    }

    fn toggle_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Timer => ActiveTab::Statistics,
            ActiveTab::Statistics => ActiveTab::Timer,
        };
    }
}
