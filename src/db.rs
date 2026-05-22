use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;
use rusqlite::{Connection, params};

use crate::timer::CompletedIteration;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = database_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create data directory {}", parent.display()))?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("failed to open database {}", path.display()))?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    pub fn insert_iteration(&self, iteration: &CompletedIteration) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO iterations (phase, duration_secs, started_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    iteration.phase.as_str(),
                    iteration.duration_secs as i64,
                    iteration.started_at.to_rfc3339(),
                    iteration.completed_at.to_rfc3339(),
                ],
            )
            .context("failed to insert completed iteration")?;
        Ok(())
    }

    fn init(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS iterations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    phase TEXT NOT NULL,
                    duration_secs INTEGER NOT NULL,
                    started_at TEXT NOT NULL,
                    completed_at TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_iterations_completed_at
                ON iterations(completed_at);

                CREATE INDEX IF NOT EXISTS idx_iterations_phase_completed_at
                ON iterations(phase, completed_at);
                ",
            )
            .context("failed to initialize database schema")?;
        Ok(())
    }
}

pub fn database_path() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("could not determine user home directory")?;
    Ok(base_dirs
        .home_dir()
        .join(".local/share/pomotimer/pomotimer.sqlite3"))
}
