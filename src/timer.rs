use std::time::Duration;

use chrono::{DateTime, Local};

use crate::config::TimersConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::ShortBreak => "short_break",
            Self::LongBreak => "long_break",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Work => "Work",
            Self::ShortBreak => "Short Break",
            Self::LongBreak => "Long Break",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletedIteration {
    pub phase: Phase,
    pub duration_secs: u64,
    pub started_at: DateTime<Local>,
    pub completed_at: DateTime<Local>,
}

#[derive(Debug)]
pub struct Timer {
    phase: Phase,
    remaining: Duration,
    running: bool,
    work_sessions_completed: u32,
    started_at: Option<DateTime<Local>>,
}

impl Timer {
    pub fn new(config: &TimersConfig) -> Self {
        Self {
            phase: Phase::Work,
            remaining: duration_for(Phase::Work, config),
            running: false,
            work_sessions_completed: 0,
            started_at: None,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn remaining(&self) -> Duration {
        self.remaining
    }

    pub fn start_pause(&mut self) {
        self.running = !self.running;
        if self.running && self.started_at.is_none() {
            self.started_at = Some(Local::now());
        }
    }

    pub fn stop(&mut self, config: &TimersConfig) {
        self.phase = Phase::Work;
        self.remaining = duration_for(self.phase, config);
        self.running = false;
        self.started_at = None;
    }

    pub fn reset(&mut self, config: &TimersConfig) {
        self.remaining = duration_for(self.phase, config);
        self.running = false;
        self.started_at = None;
    }

    pub fn next(&mut self, config: &TimersConfig) {
        if self.phase == Phase::Work {
            self.work_sessions_completed += 1;
        }
        self.advance_phase(config, false);
    }

    pub fn tick(&mut self, elapsed: Duration, config: &TimersConfig) -> Option<CompletedIteration> {
        if !self.running {
            return None;
        }

        if elapsed < self.remaining {
            self.remaining -= elapsed;
            return None;
        }

        let completed_at = Local::now();
        let completed = CompletedIteration {
            phase: self.phase,
            duration_secs: duration_for(self.phase, config).as_secs(),
            started_at: self.started_at.unwrap_or(completed_at),
            completed_at,
        };

        if self.phase == Phase::Work {
            self.work_sessions_completed += 1;
        }

        self.advance_phase(config, config.auto_start);
        Some(completed)
    }

    fn advance_phase(&mut self, config: &TimersConfig, auto_start: bool) {
        let long_break_after_sessions = config.long_break_after_sessions.max(1);
        self.phase = match self.phase {
            Phase::Work if self.work_sessions_completed % long_break_after_sessions == 0 => {
                Phase::LongBreak
            }
            Phase::Work => Phase::ShortBreak,
            Phase::ShortBreak | Phase::LongBreak => Phase::Work,
        };
        self.remaining = duration_for(self.phase, config);
        self.running = auto_start;
        self.started_at = auto_start.then(Local::now);
    }
}

fn duration_for(phase: Phase, config: &TimersConfig) -> Duration {
    let mins = match phase {
        Phase::Work => config.work_mins,
        Phase::ShortBreak => config.short_break_mins,
        Phase::LongBreak => config.long_break_mins,
    };
    Duration::from_secs(mins * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(long_break_after_sessions: u32) -> TimersConfig {
        TimersConfig {
            work_mins: 25,
            short_break_mins: 5,
            long_break_mins: 15,
            long_break_after_sessions,
            auto_start: false,
        }
    }

    #[test]
    fn manual_next_uses_short_break_before_configured_long_break() {
        let config = config(4);
        let mut timer = Timer::new(&config);

        timer.next(&config);
        assert_eq!(timer.phase(), Phase::ShortBreak);
    }

    #[test]
    fn manual_next_uses_long_break_after_configured_work_sessions() {
        let config = config(2);
        let mut timer = Timer::new(&config);

        timer.next(&config);
        assert_eq!(timer.phase(), Phase::ShortBreak);

        timer.next(&config);
        assert_eq!(timer.phase(), Phase::Work);

        timer.next(&config);
        assert_eq!(timer.phase(), Phase::LongBreak);
    }
}
