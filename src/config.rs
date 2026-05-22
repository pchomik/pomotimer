use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub timers: TimersConfig,
    pub notifications: NotificationsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimersConfig {
    pub work_mins: u64,
    pub short_break_mins: u64,
    pub long_break_mins: u64,
    #[serde(default = "default_long_break_after_sessions")]
    pub long_break_after_sessions: u32,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub enable: bool,
    pub work_done_msg: String,
    pub break_done_msg: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            timers: TimersConfig {
                work_mins: 25,
                short_break_mins: 5,
                long_break_mins: 15,
                long_break_after_sessions: default_long_break_after_sessions(),
                auto_start: false,
            },
            notifications: NotificationsConfig {
                enable: true,
                work_done_msg: "Czas na przerwę!".to_string(),
                break_done_msg: "Wracaj do pracy!".to_string(),
            },
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("could not determine user home directory")?;
    Ok(base_dirs.home_dir().join(".config/pomotimer/config.toml"))
}

pub fn load_or_create() -> Result<AppConfig> {
    let path = config_path()?;

    if !path.exists() {
        let config = AppConfig::default();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }
        let contents =
            toml::to_string_pretty(&config).context("failed to serialize default config")?;
        fs::write(&path, contents)
            .with_context(|| format!("failed to write config file {}", path.display()))?;
        return Ok(config);
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

fn default_long_break_after_sessions() -> u32 {
    4
}
