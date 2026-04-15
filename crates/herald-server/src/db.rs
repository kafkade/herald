use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

use herald_common::*;

/// Initialize the SQLite connection pool with WAL mode.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run embedded migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

// ── Messages ──────────────────────────────────────────────────────

pub async fn create_message(pool: &SqlitePool, msg: &Message) -> Result<(), sqlx::Error> {
    let grid_json = serde_json::to_string(&msg.grid).unwrap();
    let h_align = serde_json::to_string(&msg.h_align)
        .unwrap()
        .replace('"', "");
    let v_align = serde_json::to_string(&msg.v_align)
        .unwrap()
        .replace('"', "");
    let id = msg.id.to_string();
    let created = msg.created_at.to_rfc3339();
    let expires = msg.expires_at.map(|d| d.to_rfc3339());

    sqlx::query(
        "INSERT INTO messages (id, grid, h_align, v_align, queue_position, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&grid_json)
    .bind(&h_align)
    .bind(&v_align)
    .bind(msg.queue_position)
    .bind(&created)
    .bind(&expires)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_messages(pool: &SqlitePool) -> Result<Vec<Message>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM messages ORDER BY queue_position ASC, created_at ASC")
        .fetch_all(pool)
        .await?;

    Ok(rows.iter().map(row_to_message).collect())
}

pub async fn get_message(pool: &SqlitePool, id: &str) -> Result<Option<Message>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM messages WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(row.as_ref().map(row_to_message))
}

pub async fn update_message(
    pool: &SqlitePool,
    id: &str,
    req: &UpdateMessageRequest,
) -> Result<bool, sqlx::Error> {
    // Build dynamic update
    let mut sets = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref grid) = req.grid {
        sets.push("grid = ?");
        values.push(serde_json::to_string(grid).unwrap());
    }
    if let Some(ref h) = req.h_align {
        sets.push("h_align = ?");
        values.push(serde_json::to_string(h).unwrap().replace('"', ""));
    }
    if let Some(ref v) = req.v_align {
        sets.push("v_align = ?");
        values.push(serde_json::to_string(v).unwrap().replace('"', ""));
    }
    if let Some(pos) = req.queue_position {
        sets.push("queue_position = ?");
        values.push(pos.to_string());
    }
    if let Some(ref exp) = req.expires_at {
        sets.push("expires_at = ?");
        values.push(match exp {
            Some(d) => d.to_rfc3339(),
            None => String::new(),
        });
    }

    if sets.is_empty() {
        return Ok(true); // nothing to update
    }

    let sql = format!("UPDATE messages SET {} WHERE id = ?", sets.join(", "));
    let mut query = sqlx::query(&sql);

    for val in &values {
        // Handle the expires_at = null case
        if val.is_empty() && sets.iter().any(|s| s.contains("expires_at")) {
            query = query.bind(None::<String>);
        } else {
            query = query.bind(val);
        }
    }
    query = query.bind(id);

    let result = query.execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_message(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM messages WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

fn row_to_message(row: &sqlx::sqlite::SqliteRow) -> Message {
    let id_str: String = row.get("id");
    let grid_json: String = row.get("grid");
    let h_align_str: String = row.get("h_align");
    let v_align_str: String = row.get("v_align");
    let created_str: String = row.get("created_at");
    let expires_str: Option<String> = row.get("expires_at");

    Message {
        id: uuid::Uuid::parse_str(&id_str).unwrap(),
        grid: serde_json::from_str(&grid_json).unwrap(),
        h_align: serde_json::from_str(&format!("\"{h_align_str}\"")).unwrap(),
        v_align: serde_json::from_str(&format!("\"{v_align_str}\"")).unwrap(),
        queue_position: row.get("queue_position"),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&chrono::Utc),
        expires_at: expires_str.map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&chrono::Utc)
        }),
    }
}

// ── Countdowns ────────────────────────────────────────────────────

pub async fn create_countdown(pool: &SqlitePool, cd: &Countdown) -> Result<(), sqlx::Error> {
    let id = cd.id.to_string();
    let target = cd.target.to_rfc3339();
    let zero_json = serde_json::to_string(&cd.zero_behavior).unwrap();
    let created = cd.created_at.to_rfc3339();

    sqlx::query(
        "INSERT INTO countdowns (id, label, target, format_template, zero_behavior, queue_position, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&cd.label)
    .bind(&target)
    .bind(&cd.format_template)
    .bind(&zero_json)
    .bind(cd.queue_position)
    .bind(&created)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_countdowns(pool: &SqlitePool) -> Result<Vec<Countdown>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM countdowns ORDER BY queue_position ASC, created_at ASC")
        .fetch_all(pool)
        .await?;

    Ok(rows.iter().map(row_to_countdown).collect())
}

pub async fn get_countdown(pool: &SqlitePool, id: &str) -> Result<Option<Countdown>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM countdowns WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(row.as_ref().map(row_to_countdown))
}

pub async fn update_countdown(
    pool: &SqlitePool,
    id: &str,
    req: &UpdateCountdownRequest,
) -> Result<bool, sqlx::Error> {
    let mut sets = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref label) = req.label {
        sets.push("label = ?");
        values.push(label.clone());
    }
    if let Some(ref target) = req.target {
        sets.push("target = ?");
        values.push(target.to_rfc3339());
    }
    if let Some(ref template) = req.format_template {
        sets.push("format_template = ?");
        values.push(template.clone());
    }
    if let Some(ref zb) = req.zero_behavior {
        sets.push("zero_behavior = ?");
        values.push(serde_json::to_string(zb).unwrap());
    }
    if let Some(pos) = req.queue_position {
        sets.push("queue_position = ?");
        values.push(pos.to_string());
    }

    if sets.is_empty() {
        return Ok(true);
    }

    let sql = format!("UPDATE countdowns SET {} WHERE id = ?", sets.join(", "));
    let mut query = sqlx::query(&sql);
    for val in &values {
        query = query.bind(val);
    }
    query = query.bind(id);

    let result = query.execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_countdown(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM countdowns WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

fn row_to_countdown(row: &sqlx::sqlite::SqliteRow) -> Countdown {
    let id_str: String = row.get("id");
    let target_str: String = row.get("target");
    let zero_str: String = row.get("zero_behavior");
    let created_str: String = row.get("created_at");

    Countdown {
        id: uuid::Uuid::parse_str(&id_str).unwrap(),
        label: row.get("label"),
        target: chrono::DateTime::parse_from_rfc3339(&target_str)
            .unwrap()
            .with_timezone(&chrono::Utc),
        format_template: row.get("format_template"),
        zero_behavior: serde_json::from_str(&zero_str).unwrap(),
        queue_position: row.get("queue_position"),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&chrono::Utc),
    }
}

// ── Queue (derived) ───────────────────────────────────────────────

/// Build the merged queue from messages + countdowns, sorted by queue_position.
pub async fn get_queue(pool: &SqlitePool) -> Result<Vec<QueueItem>, sqlx::Error> {
    // Messages contribute queue items
    let msg_rows = sqlx::query(
        "SELECT id, queue_position, created_at, expires_at, grid FROM messages ORDER BY queue_position ASC, created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    let cd_rows = sqlx::query(
        "SELECT id, label, queue_position, created_at FROM countdowns ORDER BY queue_position ASC, created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut items: Vec<QueueItem> = Vec::new();

    for row in &msg_rows {
        let id_str: String = row.get("id");
        let grid_json: String = row.get("grid");
        let expires_str: Option<String> = row.get("expires_at");

        // Derive a label from the grid content (first non-blank chars)
        let label = derive_message_label(&grid_json);

        items.push(QueueItem {
            id: uuid::Uuid::parse_str(&id_str).unwrap(),
            kind: QueueItemKind::Message,
            label,
            queue_position: row.get("queue_position"),
            expires_at: expires_str.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .unwrap()
                    .with_timezone(&chrono::Utc)
            }),
        });
    }

    for row in &cd_rows {
        let id_str: String = row.get("id");
        items.push(QueueItem {
            id: uuid::Uuid::parse_str(&id_str).unwrap(),
            kind: QueueItemKind::Countdown,
            label: row.get("label"),
            queue_position: row.get("queue_position"),
            expires_at: None,
        });
    }

    items.sort_by(|a, b| {
        a.queue_position
            .cmp(&b.queue_position)
            .then_with(|| a.label.cmp(&b.label))
    });

    Ok(items)
}

/// Extract readable text from a grid JSON for queue labels.
fn derive_message_label(grid_json: &str) -> String {
    let grid: Grid = match serde_json::from_str(grid_json) {
        Ok(g) => g,
        Err(_) => return "(message)".to_string(),
    };

    let mut chars = Vec::new();
    for row in &grid.0 {
        for cell in row {
            match cell {
                CellContent::Char(c) => chars.push(*c),
                CellContent::Blank => {
                    if !chars.is_empty() && chars.last() != Some(&' ') {
                        chars.push(' ');
                    }
                }
                CellContent::Color(_) => {}
            }
        }
        // Row boundary adds space
        if !chars.is_empty() && chars.last() != Some(&' ') {
            chars.push(' ');
        }
    }

    let text: String = chars.into_iter().collect::<String>().trim().to_string();
    if text.is_empty() {
        "(message)".to_string()
    } else if text.len() > 44 {
        format!("{}...", &text[..41])
    } else {
        text
    }
}

/// Reorder queue items: rewrite queue_position for both messages and countdowns.
pub async fn reorder_queue(pool: &SqlitePool, order: &[uuid::Uuid]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for (pos, id) in order.iter().enumerate() {
        let id_str = id.to_string();
        let pos = pos as i64;

        // Try updating in messages first
        let result = sqlx::query("UPDATE messages SET queue_position = ? WHERE id = ?")
            .bind(pos)
            .bind(&id_str)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            // Must be a countdown
            sqlx::query("UPDATE countdowns SET queue_position = ? WHERE id = ?")
                .bind(pos)
                .bind(&id_str)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

// ── Configuration ─────────────────────────────────────────────────

pub async fn get_config(
    pool: &SqlitePool,
) -> Result<serde_json::Map<String, serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query("SELECT key, value FROM configuration ORDER BY key")
        .fetch_all(pool)
        .await?;

    let mut map = serde_json::Map::new();
    for row in &rows {
        let key: String = row.get("key");
        let value: String = row.get("value");
        // Try to parse as JSON value, fall back to string
        let json_val = serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
        map.insert(key, json_val);
    }
    Ok(map)
}

pub async fn set_config(
    pool: &SqlitePool,
    values: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();

    for (key, value) in values {
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };

        sqlx::query(
            "INSERT INTO configuration (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(&val_str)
        .bind(&now)
        .execute(pool)
        .await?;
    }
    Ok(())
}

// ── Rotation state ────────────────────────────────────────────────

pub async fn get_current_index(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT current_index FROM rotation_state WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok(row.get("current_index"))
}

/// Get the next auto-assigned queue position (max + 1).
pub async fn next_queue_position(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COALESCE(MAX(pos), -1) as max_pos FROM (
            SELECT queue_position as pos FROM messages
            UNION ALL
            SELECT queue_position as pos FROM countdowns
        )",
    )
    .fetch_one(pool)
    .await?;
    let max: i64 = row.get("max_pos");
    Ok(max + 1)
}

/// Count total queue items.
pub async fn queue_size(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let row = sqlx::query(
        "SELECT (SELECT COUNT(*) FROM messages) + (SELECT COUNT(*) FROM countdowns) as total",
    )
    .fetch_one(pool)
    .await?;
    let total: i64 = row.get("total");
    Ok(total as usize)
}
