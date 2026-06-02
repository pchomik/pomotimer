use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, NaiveDate};
use directories::BaseDirs;
use rusqlite::{Connection, params};

use crate::timer::CompletedIteration;

#[derive(Debug, Clone)]
pub struct DailyAgg {
    pub date: NaiveDate,
    pub total_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct MonthlyAgg {
    pub month: u32,
    pub total_seconds: u64,
}

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

    pub fn daily_window(&self, start: NaiveDate, end: NaiveDate) -> Result<Vec<DailyAgg>> {
        let end_exclusive = end.succ_opt().unwrap_or(end);
        let mut stmt = self.conn.prepare(
            "SELECT duration_secs, started_at, completed_at
             FROM iterations
             WHERE completed_at >= ?1 AND started_at < ?2
             ORDER BY started_at ASC",
        )?;

        let rows = stmt.query_map(
            params![
                start.format("%Y-%m-%d").to_string(),
                end_exclusive.format("%Y-%m-%d").to_string(),
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let mut map: BTreeMap<NaiveDate, u64> = BTreeMap::new();

        for row in rows {
            let (duration_secs, started_str, completed_str) = row?;
            let started_at = DateTime::parse_from_rfc3339(&started_str)
                .map(|dt| dt.with_timezone(&Local))
                .context("failed to parse started_at")?;
            let completed_at = DateTime::parse_from_rfc3339(&completed_str)
                .map(|dt| dt.with_timezone(&Local))
                .context("failed to parse completed_at")?;

            for (date, secs) in split_seconds_across_days(started_at, completed_at, duration_secs) {
                if date >= start && date <= end {
                    *map.entry(date).or_insert(0) += secs;
                }
            }
        }

        let mut date = start;
        let mut result = Vec::new();
        while date <= end {
            let total_seconds = map.get(&date).copied().unwrap_or(0);
            result.push(DailyAgg {
                date,
                total_seconds,
            });
            date = date.succ_opt().unwrap_or(date);
        }

        Ok(result)
    }

    pub fn monthly_window(
        &self,
        start_month_first: NaiveDate,
        end_month_first: NaiveDate,
    ) -> Result<Vec<MonthlyAgg>> {
        let start = start_month_first;
        let end = next_month_start(end_month_first);

        let mut stmt = self.conn.prepare(
            "SELECT duration_secs, completed_at
             FROM iterations
             WHERE completed_at >= ?1 AND completed_at < ?2
             ORDER BY completed_at ASC",
        )?;

        let rows = stmt.query_map(
            params![
                start.format("%Y-%m-%d").to_string(),
                end.format("%Y-%m-%d").to_string(),
            ],
            |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?)),
        )?;

        let mut map: BTreeMap<(i32, u32), u64> = BTreeMap::new();

        for row in rows {
            let (duration_secs, completed_str) = row?;
            let completed_at = DateTime::parse_from_rfc3339(&completed_str)
                .map(|dt| dt.with_timezone(&Local))
                .context("failed to parse completed_at")?;
            let date = completed_at.date_naive();
            let key = (date.year(), date.month());
            *map.entry(key).or_insert(0) += duration_secs;
        }

        let mut current = start_month_first;
        let mut result = Vec::new();
        while current < end_month_first.succ_opt().unwrap_or(end_month_first) {
            let key = (current.year(), current.month());
            let total_seconds = map.get(&key).copied().unwrap_or(0);
            result.push(MonthlyAgg {
                month: key.1,
                total_seconds,
            });
            current = next_month_start(current);
        }

        Ok(result)
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

fn split_seconds_across_days(
    started_at: DateTime<Local>,
    completed_at: DateTime<Local>,
    duration_secs: u64,
) -> Vec<(NaiveDate, u64)> {
    let start_date = started_at.date_naive();
    let end_date = completed_at.date_naive();

    if start_date == end_date {
        return vec![(start_date, duration_secs)];
    }

    let mut result = Vec::new();
    let midnight_after_start = start_date
        .succ_opt()
        .unwrap_or(start_date)
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let midnight_after_start: DateTime<Local> = midnight_after_start
        .and_local_timezone(Local)
        .earliest()
        .unwrap_or(started_at);

    let secs_in_first_day = (midnight_after_start - started_at)
        .num_seconds()
        .min(duration_secs as i64) as u64;

    result.push((start_date, secs_in_first_day));

    let mut remaining = duration_secs - secs_in_first_day;
    let mut current_date = start_date.succ_opt().unwrap_or(start_date);

    while remaining > 0 && current_date < end_date {
        let full_day_secs = 86400.min(remaining);
        result.push((current_date, full_day_secs));
        remaining -= full_day_secs;
        current_date = current_date.succ_opt().unwrap_or(current_date);
    }

    if remaining > 0 {
        result.push((end_date, remaining));
    }

    result
}

fn next_month_start(date: NaiveDate) -> NaiveDate {
    let month = date.month();
    let year = date.year();
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    }
}

pub fn database_path() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("could not determine user home directory")?;
    Ok(base_dirs
        .home_dir()
        .join(".local/share/pomotimer/pomotimer.sqlite3"))
}
