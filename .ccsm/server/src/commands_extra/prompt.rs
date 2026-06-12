//! Prompt management.
//!
//! The upstream `cc_switch_lib::Prompt` type lives in a private module
//! so we cannot name it from the dispatch layer. Instead we operate on
//! the `prompts` table directly via SQL and read the result back as
//! JSON. This mirrors the proxy / stream_check shim pattern.

use super::{require_app_str, require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

const COLUMNS: &str = "id, app_type, name, content, description, enabled, created_at, updated_at";

pub async fn get_prompts(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = require_app_str(&args)?;
    let conn = open_db(ctx)?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM prompts WHERE app_type = ?1 ORDER BY created_at ASC, id ASC"
        ))
        .map_err(|e| ApiError::Internal(format!("prepare get_prompts: {e}")))?;
    let entries: Vec<Value> = stmt
        .query_map([app.as_str()], row_to_json)
        .map_err(|e| ApiError::Internal(format!("query prompts: {e}")))?
        .filter_map(|r| r.ok())
        .collect();
    // IndexMap shape: a JSON object keyed by id.
    let mut map: serde_json::Map<String, Value> = serde_json::Map::new();
    for entry in entries {
        if let Some(id) = entry.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            map.insert(id, entry);
        }
    }
    Ok(Value::Object(map))
}

pub async fn upsert_prompt(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = require_app_str(&args)?;
    let id: String = require_arg(&args, "id")?;
    let prompt: Value = require_arg(&args, "prompt")?;
    let name = prompt
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    let content = prompt
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let description = prompt
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let enabled = prompt
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let now = chrono::Utc::now().timestamp();
    let conn = open_db(ctx)?;
    conn.execute(
        "INSERT INTO prompts (id, app_type, name, content, description, enabled, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7) \
         ON CONFLICT(id, app_type) DO UPDATE SET \
            name = excluded.name, \
            content = excluded.content, \
            description = excluded.description, \
            enabled = excluded.enabled, \
            updated_at = excluded.updated_at",
        rusqlite::params![id, app.as_str(), name, content, description, enabled, now],
    )
    .map_err(|e| ApiError::Internal(format!("upsert_prompt: {e}")))?;
    Ok(Value::Null)
}

pub async fn delete_prompt(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = require_app_str(&args)?;
    let id: String = require_arg(&args, "id")?;
    let conn = open_db(ctx)?;
    conn.execute(
        "DELETE FROM prompts WHERE id = ?1 AND app_type = ?2",
        rusqlite::params![id, app.as_str()],
    )
    .map_err(|e| ApiError::Internal(format!("delete_prompt: {e}")))?;
    Ok(Value::Null)
}

pub async fn enable_prompt(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = require_app_str(&args)?;
    let id: String = require_arg(&args, "id")?;
    let now = chrono::Utc::now().timestamp();
    let conn = open_db(ctx)?;
    let affected = conn
        .execute(
            "UPDATE prompts SET enabled = 1, updated_at = ?1 WHERE id = ?2 AND app_type = ?3",
            rusqlite::params![now, id, app.as_str()],
        )
        .map_err(|e| ApiError::Internal(format!("enable_prompt: {e}")))?;
    if affected == 0 {
        return Err(ApiError::Internal(format!(
            "prompt {id} not found for app {}",
            app.as_str()
        )));
    }
    Ok(Value::Null)
}

pub async fn import_prompt_from_file(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    // The upstream "import" reads CLAUDE.md / AGENTS.md from the user's
    // home (or app config dir). We replicate the behaviour minimally by
    // looking up the conventional prompt file path for the app and
    // storing its content as a new prompt.
    let app = require_app_str(&args)?;
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let file_name = match app {
        cc_switch_lib::AppType::Claude => "CLAUDE.md",
        cc_switch_lib::AppType::Codex => "AGENTS.md",
        cc_switch_lib::AppType::Gemini => "GEMINI.md",
        _ => "PROMPT.md",
    };
    let path = home.join(file_name);
    let content = std::fs::read_to_string(&path).ok();
    let Some(content) = content else {
        return Ok(Value::String(String::new()));
    };
    let name = file_name.to_string();
    let id = format!("imported-{}-{}", app.as_str(), chrono::Utc::now().timestamp());
    let now = chrono::Utc::now().timestamp();
    let conn = open_db(ctx)?;
    conn.execute(
        "INSERT INTO prompts (id, app_type, name, content, description, enabled, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, NULL, 1, ?5, ?5)",
        rusqlite::params![id, app.as_str(), name, content, now],
    )
    .map_err(|e| ApiError::Internal(format!("import_prompt_from_file: {e}")))?;
    Ok(Value::String(id))
}

pub async fn get_current_prompt_file_content(args: Value) -> Result<Value> {
    let app = require_app_str(&args)?;
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let file_name = match app {
        cc_switch_lib::AppType::Claude => "CLAUDE.md",
        cc_switch_lib::AppType::Codex => "AGENTS.md",
        cc_switch_lib::AppType::Gemini => "GEMINI.md",
        _ => "PROMPT.md",
    };
    let path = home.join(file_name);
    let content = std::fs::read_to_string(&path).ok();
    Ok(content.map(Value::String).unwrap_or(Value::Null))
}

fn row_to_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let id: String = row.get(0)?;
    let app_type: String = row.get(1)?;
    let name: String = row.get(2)?;
    let content: String = row.get(3)?;
    let description: Option<String> = row.get(4)?;
    let enabled: bool = row.get(5)?;
    let created_at: Option<i64> = row.get(6)?;
    let updated_at: Option<i64> = row.get(7)?;
    Ok(serde_json::json!({
        "id": id,
        "appType": app_type,
        "name": name,
        "content": content,
        "description": description,
        "enabled": enabled,
        "createdAt": created_at,
        "updatedAt": updated_at,
    }))
}

fn open_db(ctx: &Arc<AppContext>) -> Result<rusqlite::Connection> {
    let path = ctx
        .opts
        .data_dir
        .join(".cc-switch")
        .join("cc-switch.db");
    rusqlite::Connection::open(&path)
        .map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}
