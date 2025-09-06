use anyhow::Result;
use tauri::AppHandle;
use tauri::Manager;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::import::ParsedFile;
use std::path::{PathBuf};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub entry_date: DateTime<Utc>,
    pub entry_timezone: String,
    pub source_path: String,
    pub source_type: String,
    pub text_hash: String,
    pub embedding: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sentiment: Option<f32>,
    pub language: Option<String>,
}

pub async fn init_database(app_handle: &AppHandle) -> Result<()> {
    let _ = std::fs::create_dir_all(get_db_dir(app_handle)?);
    let conn = open_conn(app_handle)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS entries (
            id TEXT PRIMARY KEY,
            title TEXT,
            body TEXT NOT NULL,
            entry_date TEXT NOT NULL,
            entry_timezone TEXT NOT NULL,
            source_path TEXT NOT NULL,
            source_type TEXT NOT NULL,
            text_hash TEXT NOT NULL UNIQUE,
            embedding BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            sentiment REAL,
            language TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_entries_entry_date ON entries(entry_date);
        CREATE INDEX IF NOT EXISTS idx_entries_text_hash ON entries(text_hash);

        -- Full-text search virtual table
        CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts
        USING fts5(
            title,
            body,
            entry_id UNINDEXED
        );

        -- Settings table (key/value)
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#
    )?;
    Ok(())
}

// Helper: app data dir
fn get_db_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    match app_handle.path().app_data_dir() {
        Ok(mut dir) => {
            dir.push("journal-reader");
            Ok(dir)
        }
        Err(_) => {
            // Fallback: current working directory
            Ok(std::env::current_dir()?)
        }
    }
}

fn get_db_file_path(app_handle: &AppHandle) -> Result<PathBuf> {
    let mut path = get_db_dir(app_handle)?;
    path.push("journal.db");
    Ok(path)
}

fn open_conn(app_handle: &AppHandle) -> Result<Connection> {
    let db_path = get_db_file_path(app_handle)?;
    let conn = Connection::open(db_path)?;
    Ok(conn)
}

pub async fn save_entry(
    app_handle: &AppHandle,
    parsed_file: ParsedFile,
    entry_date: DateTime<Utc>,
    entry_timezone: String,
) -> Result<String> {
    let entry_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(existing_id) = check_duplicate(app_handle, &parsed_file.text_hash).await? {
        return Err(anyhow::anyhow!(
            "Duplicate content found (existing entry: {})", 
            existing_id
        ));
    }
    
    let now = Utc::now().to_rfc3339();
    let conn = open_conn(app_handle)?;
    conn.execute(
        r#"INSERT INTO entries (
            id, title, body, entry_date, entry_timezone, source_path, source_type, text_hash,
            embedding, created_at, updated_at, sentiment, language
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10, NULL, NULL)"#,
        params![
            entry_id,
            parsed_file.title,
            parsed_file.content,
            entry_date.to_rfc3339(),
            entry_timezone,
            parsed_file.path,
            parsed_file.file_type.as_str(),
            parsed_file.text_hash,
            now,
            now,
        ],
    )?;

    // Insert into FTS index
    conn.execute(
        r#"INSERT INTO entries_fts (title, body, entry_id) VALUES (?1, ?2, ?3)"#,
        params![
            parsed_file.title.clone().unwrap_or_default(),
            parsed_file.content.clone(),
            entry_id.clone()
        ],
    )?;

    eprintln!("[db] saved entry id={} path={} date={} tz={}", entry_id, parsed_file.path, entry_date, entry_timezone);

    Ok(entry_id)
}

pub async fn check_duplicate(app_handle: &AppHandle, text_hash: &str) -> Result<Option<String>> {
    let conn = open_conn(app_handle)?;
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM entries WHERE text_hash = ?1 LIMIT 1",
            params![text_hash],
            |row| row.get(0),
        )
        .optional()?;
    Ok(id)
}

// Import jobs removed in simplified flow (we import synchronously)

pub async fn list_entries_by_month(
    app_handle: &AppHandle,
    year: i32,
    month: u32,
) -> Result<Vec<Entry>> {
    let conn = open_conn(app_handle)?;
    let start = format!("{:04}-{:02}-01T00:00:00Z", year, month);
    // next month
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let end = format!("{:04}-{:02}-01T00:00:00Z", ny, nm);

    let mut stmt = conn.prepare(
        r#"SELECT id, title, body, entry_date, entry_timezone, source_path, source_type, text_hash,
                   created_at, updated_at, sentiment, language
            FROM entries
            WHERE entry_date >= ?1 AND entry_date < ?2
            ORDER BY entry_date ASC"#,
    )?;

    let rows = stmt.query_map(params![start, end], |row| {
        let entry_date_str: String = row.get(3)?;
        let entry_date = DateTime::parse_from_rfc3339(&entry_date_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(Entry {
            id: row.get(0)?,
            title: row.get(1)?,
            body: row.get(2)?,
            entry_date,
            entry_timezone: row.get(4)?,
            source_path: row.get(5)?,
            source_type: row.get(6)?,
            text_hash: row.get(7)?,
            embedding: None,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                .map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                .map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            sentiment: row.get(10).ok(),
            language: row.get(11).ok(),
        })
    })?;

    let mut entries = Vec::new();
    for r in rows { entries.push(r?); }
    Ok(entries)
}

pub async fn get_entry_by_id(app_handle: &AppHandle, entry_id: &str) -> Result<Option<Entry>> {
    let conn = open_conn(app_handle)?;
    let mut stmt = conn.prepare(
        r#"SELECT id, title, body, entry_date, entry_timezone, source_path, source_type, text_hash,
                   created_at, updated_at, sentiment, language
            FROM entries WHERE id = ?1"#,
    )?;
    let row = stmt.query_row(params![entry_id], |row| {
        let entry_date_str: String = row.get(3)?;
        let entry_date = DateTime::parse_from_rfc3339(&entry_date_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(Entry {
            id: row.get(0)?,
            title: row.get(1)?,
            body: row.get(2)?,
            entry_date,
            entry_timezone: row.get(4)?,
            source_path: row.get(5)?,
            source_type: row.get(6)?,
            text_hash: row.get(7)?,
            embedding: None,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                .map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                .map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            sentiment: row.get(10).ok(),
            language: row.get(11).ok(),
        })
    }).optional()?;
    Ok(row)
}

// Simplified app: no FTS at this stage
pub async fn search_entries_fts_simple(
    app_handle: &AppHandle,
    query: &str,
    limit: u32,
) -> Result<Vec<(Entry, String)>> {
    if query.trim().is_empty() { return Ok(vec![]); }
    let db_path = get_db_file_path(app_handle)?;
    let q = query.to_string();
    let lim = limit as i64;
    let results = tokio::task::spawn_blocking(move || -> Result<Vec<(Entry, String)>> {
        // rudimentary tracing
        eprintln!("[fts] open db");
        let conn = Connection::open(db_path)?;
        eprintln!("[fts] prepare statement");
        let mut stmt = conn.prepare(
            r#"SELECT 
                    e.id, e.title, e.body, e.entry_date, e.entry_timezone, e.source_path, e.source_type, e.text_hash,
                    e.created_at, e.updated_at, e.sentiment, e.language,
                    snippet(entries_fts, 1, '', '', '...', 10) AS snip
                FROM entries_fts f
                JOIN entries e ON e.id = f.entry_id
                WHERE entries_fts MATCH ?1
                ORDER BY bm25(entries_fts) ASC
                LIMIT ?2"#,
        )?;

        eprintln!("[fts] execute query");
        let rows = stmt.query_map(params![q, lim], |row| {
            let entry_date_str: String = row.get(3)?;
            let entry_date = DateTime::parse_from_rfc3339(&entry_date_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let created_at_str: String = row.get(8)?;
            let updated_at_str: String = row.get(9)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let entry = Entry {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                entry_date,
                entry_timezone: row.get(4)?,
                source_path: row.get(5)?,
                source_type: row.get(6)?,
                text_hash: row.get(7)?,
                embedding: None,
                created_at,
                updated_at,
                sentiment: row.get(10).ok(),
                language: row.get(11).ok(),
            };
            let snip: String = row.get(12)?;
            Ok((entry, snip))
        })?;

        let mut results = Vec::new();
        for r in rows { results.push(r?); }
        eprintln!("[fts] rows={} ", results.len());
        Ok(results)
    })
    .await
    .map_err(|e| anyhow::anyhow!(e.to_string()))??;

    Ok(results)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbInfo {
    pub db_path: String,
    pub total_entries: u32,
    pub years: Vec<i32>,
}

pub async fn get_db_info(app_handle: &AppHandle) -> Result<DbInfo> {
    let path = get_db_file_path(app_handle)?;
    let conn = open_conn(app_handle)?;
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0)).unwrap_or(0);
    let years = get_available_years(app_handle).await.unwrap_or_default();
    Ok(DbInfo {
        db_path: path.to_string_lossy().to_string(),
        total_entries: total as u32,
        years,
    })
}

pub async fn ensure_fts_populated(app_handle: &AppHandle) -> Result<()> {
    let conn = open_conn(app_handle)?;
    // Create FTS table if missing (idempotent)
    conn.execute_batch(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts
        USING fts5(
            title,
            body,
            entry_id UNINDEXED
        );
        "#,
    )?;

    // Backfill any missing rows into FTS from entries
    conn.execute(
        r#"INSERT INTO entries_fts (title, body, entry_id)
            SELECT IFNULL(title, ''), body, id
            FROM entries e
            WHERE NOT EXISTS (
                SELECT 1 FROM entries_fts f WHERE f.entry_id = e.id
            )"#,
        [],
    )?;

    Ok(())
}

pub async fn get_settings(app_handle: &AppHandle) -> Result<Vec<(String, String)>> {
    let conn = open_conn(app_handle)?;
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        let k: String = row.get(0)?;
        let v: String = row.get(1)?;
        Ok((k, v))
    })?;
    let mut items = Vec::new();
    for r in rows { items.push(r?); }

    // Supply defaults if missing
    let mut have = std::collections::HashSet::new();
    for (k, _) in &items { have.insert(k.clone()); }
    let defaults = vec![
        ("ai_provider".to_string(), "ollama".to_string()),
        ("ollama_url".to_string(), "http://localhost:11434".to_string()),
        ("default_model".to_string(), "llama3.1:8b".to_string()),
        ("embedding_model".to_string(), "nomic-embed-text".to_string()),
    ];
    for (k, v) in defaults {
        if !have.contains(&k) {
            items.push((k, v));
        }
    }

    Ok(items)
}

pub async fn update_setting(app_handle: &AppHandle, key: &str, value: &str) -> Result<()> {
    let conn = open_conn(app_handle)?;
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonthCount {
    pub month: u32,
    pub count: u32,
}

pub async fn get_available_years(app_handle: &AppHandle) -> Result<Vec<i32>> {
    let conn = open_conn(app_handle)?;
    let mut stmt = conn.prepare(
        r#"SELECT DISTINCT substr(entry_date, 1, 4) as year
            FROM entries
            ORDER BY year DESC"#,
    )?;
    let rows = stmt.query_map([], |row| {
        let year_str: String = row.get(0)?;
        let year = year_str.parse::<i32>().unwrap_or(0);
        Ok(year)
    })?;
    let mut years = Vec::new();
    for r in rows { years.push(r?); }
    Ok(years)
}

pub async fn get_month_counts_for_year(app_handle: &AppHandle, year: i32) -> Result<Vec<MonthCount>> {
    let conn = open_conn(app_handle)?;
    let start = format!("{:04}-01-01T00:00:00Z", year);
    let end = format!("{:04}-12-31T23:59:59Z", year);
    let mut stmt = conn.prepare(
        r#"SELECT cast(substr(entry_date, 6, 2) as INTEGER) as month,
                   count(*) as cnt
            FROM entries
            WHERE entry_date BETWEEN ?1 AND ?2
            GROUP BY month
            ORDER BY month ASC"#,
    )?;
    let rows = stmt.query_map(params![start, end], |row| {
        Ok(MonthCount { month: row.get::<_, i64>(0)? as u32, count: row.get::<_, i64>(1)? as u32 })
    })?;
    let mut counts = vec![MonthCount { month: 1, count: 0 }, MonthCount { month: 2, count: 0 }, MonthCount { month: 3, count: 0 }, MonthCount { month: 4, count: 0 }, MonthCount { month: 5, count: 0 }, MonthCount { month: 6, count: 0 }, MonthCount { month: 7, count: 0 }, MonthCount { month: 8, count: 0 }, MonthCount { month: 9, count: 0 }, MonthCount { month: 10, count: 0 }, MonthCount { month: 11, count: 0 }, MonthCount { month: 12, count: 0 }];
    for r in rows {
        let m = r?;
        if (1..=12).contains(&m.month) {
            let MonthCount { month, count } = m;
            counts[(month - 1) as usize] = MonthCount { month, count };
        }
    }
    Ok(counts)
}