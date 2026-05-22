use anyhow::{Context, Result};
use notify_rust::{Notification, Timeout, Urgency};

use crate::{
    config::NotificationsConfig,
    timer::{CompletedIteration, Phase},
};

#[derive(Debug, Clone)]
pub struct Notifier {
    config: NotificationsConfig,
}

impl Notifier {
    pub fn new(config: NotificationsConfig) -> Self {
        Self { config }
    }

    pub fn notify_completed(&self, iteration: &CompletedIteration) -> Result<()> {
        if !self.config.enable {
            return Ok(());
        }

        let body = match iteration.phase {
            Phase::Work => &self.config.work_done_msg,
            Phase::ShortBreak | Phase::LongBreak => &self.config.break_done_msg,
        };

        Notification::new()
            .summary("Pomotimer")
            .body(body)
            .urgency(Urgency::Critical)
            .timeout(Timeout::Never)
            .show()
            .context("failed to send desktop notification")?;
        Ok(())
    }
}
