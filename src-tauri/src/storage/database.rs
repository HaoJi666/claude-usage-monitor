use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: String,
    pub platform: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageRecord {
    pub id: i64,
    pub account_id: String,
    pub five_hour_utilization: Option<f64>,
    pub five_hour_resets_at: Option<String>,
    pub seven_day_utilization: Option<f64>,
    pub seven_day_resets_at: Option<String>,
    pub fetched_at: String,
}

pub fn initialize(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL DEFAULT 'claude',
            name TEXT NOT NULL DEFAULT 'Default Account',
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS usage_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id TEXT NOT NULL,
            five_hour_utilization REAL,
            five_hour_resets_at TEXT,
            seven_day_utilization REAL,
            seven_day_resets_at TEXT,
            fetched_at TEXT NOT NULL,
            FOREIGN KEY (account_id) REFERENCES accounts(id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    ).context("Failed to initialize database schema")?;
    Ok(())
}

pub fn create_account(conn: &Connection, platform: &str, name: &str) -> Result<Account> {
    let account = Account {
        id: Uuid::new_v4().to_string(),
        platform: platform.to_string(),
        name: name.to_string(),
        is_active: true,
        created_at: Utc::now().to_rfc3339(),
    };
    conn.execute(
        "INSERT INTO accounts (id, platform, name, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![account.id, account.platform, account.name, account.is_active as i32, account.created_at],
    ).context("Failed to insert account")?;
    Ok(account)
}

pub fn get_accounts(conn: &Connection) -> Result<Vec<Account>> {
    let mut stmt = conn.prepare(
        "SELECT id, platform, name, is_active, created_at FROM accounts ORDER BY created_at DESC",
    )?;
    let accounts = stmt.query_map([], |row| {
        Ok(Account {
            id: row.get(0)?,
            platform: row.get(1)?,
            name: row.get(2)?,
            is_active: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
        })
    })?
    .collect::<std::result::Result<Vec<_>, _>>()
    .context("Failed to fetch accounts")?;
    Ok(accounts)
}

pub fn delete_account(conn: &Connection, account_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM usage_history WHERE account_id = ?1",
        params![account_id],
    )?;
    conn.execute("DELETE FROM accounts WHERE id = ?1", params![account_id])?;
    Ok(())
}

pub fn save_usage_record(
    conn: &Connection,
    account_id: &str,
    five_hour_utilization: Option<f64>,
    five_hour_resets_at: Option<&str>,
    seven_day_utilization: Option<f64>,
    seven_day_resets_at: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO usage_history (account_id, five_hour_utilization, five_hour_resets_at, seven_day_utilization, seven_day_resets_at, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            account_id,
            five_hour_utilization,
            five_hour_resets_at,
            seven_day_utilization,
            seven_day_resets_at,
            Utc::now().to_rfc3339()
        ],
    ).context("Failed to save usage record")?;
    // Keep only last 1000 records per account
    conn.execute(
        "DELETE FROM usage_history WHERE account_id = ?1 AND id NOT IN (
            SELECT id FROM usage_history WHERE account_id = ?1 ORDER BY fetched_at DESC LIMIT 1000
        )",
        params![account_id],
    )?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("DB error: {}", e)),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    ).context("Failed to save setting")?;
    Ok(())
}
