//! Codex 官方历史会话统一开关迁移备份/还原。
//!
//! 上游 `src-tauri/src/codex_history_migration.rs` 是私有模块，无法从
//! `cc_switch_lib` 外部调用，因此这里保留一个与上游语义等价的最小实现。
//! 该实现只处理 `~/.codex` 下的本地历史数据，不依赖 Tauri 运行时。

use crate::error::{ApiError, Result};
use rusqlite::{backup::Backup, Connection};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const OFFICIAL_UNIFY_MIGRATION_NAME: &str = "codex-official-history-unify-v1";
const OFFICIAL_UNIFY_RESTORE_BACKUP_NAME: &str = "codex-official-history-unify-restore-v1";
const CODEX_STATE_DB_FILENAME: &str = "state_5.sqlite";
const OFFICIAL_OPENAI_CODEX_MODEL_PROVIDER_ID: &str = "openai";
const CC_SWITCH_CODEX_MODEL_PROVIDER_ID: &str = "custom";
const STATE_DB_ID_CHUNK: usize = 500;

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub restored_jsonl_files: usize,
    pub restored_state_rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

/// 是否存在统一会话开关的迁移备份。
pub async fn has_backup(_ctx: &crate::AppContext) -> Result<Value> {
    let codex_dir = codex_config_dir();
    let backup_parent = official_history_unify_backup_parent();
    Ok(Value::Bool(has_backup_for_dir(
        &backup_parent,
        &canonical_dir_string(&codex_dir),
    )))
}

/// 按迁移备份账本把当时迁入共享桶的官方会话还原回 "openai" 桶。
pub async fn restore(_ctx: &crate::AppContext) -> Result<Value> {
    let outcome = tokio::task::spawn_blocking(restore_official_history_from_backups)
        .await
        .map_err(|e| ApiError::Internal(format!("restore task panicked: {e}")))?;

    let outcome = outcome?;
    if let Some(reason) = &outcome.skipped_reason {
        log::debug!("Codex official history restore skipped: {reason}");
    } else {
        log::info!(
            "Codex official history restored from backups: jsonl_files={}, state_rows={}",
            outcome.restored_jsonl_files,
            outcome.restored_state_rows
        );
    }

    Ok(serde_json::to_value(&outcome)?)
}

fn restore_official_history_from_backups() -> Result<RestoreOutcome> {
    let codex_dir = codex_config_dir();
    let ledger_parent = official_history_unify_backup_parent();
    let restore_backup_root = migration_backup_root(OFFICIAL_UNIFY_RESTORE_BACKUP_NAME);
    let config_text = read_codex_config_text();

    restore_official_history_inner(
        &codex_dir,
        &ledger_parent,
        &restore_backup_root,
        &config_text,
    )
}

fn restore_official_history_inner(
    codex_dir: &Path,
    ledger_parent: &Path,
    restore_backup_root: &Path,
    config_text: &str,
) -> Result<RestoreOutcome> {
    let codex_dir_key = canonical_dir_string(codex_dir);
    let (official_session_ids, official_thread_ids) =
        collect_official_ledger(ledger_parent, &codex_dir_key)?;
    if official_session_ids.is_empty() && official_thread_ids.is_empty() {
        return Ok(RestoreOutcome {
            skipped_reason: Some("no_backup_ledger".to_string()),
            ..Default::default()
        });
    }

    let mut files = Vec::new();
    collect_jsonl_files(&codex_dir.join("sessions"), &mut files, 0, 8);
    collect_jsonl_files(&codex_dir.join("archived_sessions"), &mut files, 0, 4);
    let mut restored_jsonl_files = 0;
    for file_path in files {
        if rewrite_codex_session_file_lines(&file_path, codex_dir, restore_backup_root, |line| {
            rewrite_codex_session_meta_line_for_restore(line, &official_session_ids)
        })? {
            restored_jsonl_files += 1;
        }
    }

    let mut restored_state_rows = 0;
    for db_path in codex_state_db_paths(codex_dir, config_text) {
        restored_state_rows += restore_codex_state_db_official_threads(
            &db_path,
            codex_dir,
            &official_thread_ids,
            restore_backup_root,
        )?;
    }

    if restored_jsonl_files == 0 && restored_state_rows == 0 {
        return Ok(RestoreOutcome {
            skipped_reason: Some("nothing_to_restore".to_string()),
            ..Default::default()
        });
    }

    Ok(RestoreOutcome {
        restored_jsonl_files,
        restored_state_rows,
        skipped_reason: None,
    })
}

fn has_backup_for_dir(ledger_parent: &Path, codex_dir_key: &str) -> bool {
    let Ok(entries) = fs::read_dir(ledger_parent) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let generation = entry.path();
        generation.is_dir() && backup_generation_matches_dir(&generation, codex_dir_key)
    })
}

fn collect_official_ledger(
    ledger_parent: &Path,
    codex_dir_key: &str,
) -> Result<(HashSet<String>, BTreeSet<String>)> {
    let mut session_ids = HashSet::new();
    let mut thread_ids = BTreeSet::new();
    let entries = match fs::read_dir(ledger_parent) {
        Ok(entries) => entries,
        Err(_) => return Ok((session_ids, thread_ids)),
    };
    for entry in entries.flatten() {
        let generation = entry.path();
        if !generation.is_dir() {
            continue;
        }
        if !backup_generation_matches_dir(&generation, codex_dir_key) {
            continue;
        }
        let mut backup_files = Vec::new();
        collect_jsonl_files(&generation.join("jsonl"), &mut backup_files, 0, 10);
        for backup_file in backup_files {
            collect_official_session_ids_from_backup(&backup_file, &mut session_ids);
        }
        let mut backup_dbs = Vec::new();
        collect_files_with_extension(&generation.join("state"), "sqlite", &mut backup_dbs, 0, 4);
        for backup_db in backup_dbs {
            collect_official_thread_ids_from_backup(&backup_db, &mut thread_ids)?;
        }
    }
    Ok((session_ids, thread_ids))
}

fn backup_generation_matches_dir(generation: &Path, codex_dir_key: &str) -> bool {
    let Ok(text) = fs::read_to_string(generation.join("meta.json")) else {
        // 早期版本备份没有 meta.json；那个时期不存在切换 codex 目录的场景，
        // 宽容接受即可。
        return true;
    };
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| {
            value
                .get("codexConfigDir")
                .and_then(Value::as_str)
                .map(|dir| dir == codex_dir_key)
        })
        .unwrap_or(true)
}

fn collect_official_session_ids_from_backup(path: &Path, session_ids: &mut HashSet<String>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        if !line.contains("\"session_meta\"") || !line.contains("\"model_provider\"") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("model_provider").and_then(Value::as_str)
            != Some(OFFICIAL_OPENAI_CODEX_MODEL_PROVIDER_ID)
        {
            continue;
        }
        if let Some(session_id) = payload.get("id").and_then(Value::as_str) {
            session_ids.insert(session_id.to_string());
        }
    }
}

fn collect_official_thread_ids_from_backup(
    db_path: &Path,
    thread_ids: &mut BTreeSet<String>,
) -> Result<()> {
    let conn =
        match Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(conn) => conn,
            Err(_) => return Ok(()),
        };
    if !table_exists(&conn, "threads")? || !has_column(&conn, "threads", "model_provider")? {
        return Ok(());
    }
    let mut stmt = conn
        .prepare("SELECT id FROM threads WHERE model_provider = ?1")
        .map_err(|e| ApiError::Internal(format!("prepare backup db: {e}")))?;
    let rows = stmt.query_map([OFFICIAL_OPENAI_CODEX_MODEL_PROVIDER_ID], |row| {
        row.get::<_, String>(0)
    });
    if let Ok(rows) = rows {
        for thread_id in rows.flatten() {
            thread_ids.insert(thread_id);
        }
    }
    Ok(())
}

fn rewrite_codex_session_meta_line_for_restore(
    line: &str,
    official_session_ids: &HashSet<String>,
) -> Option<String> {
    if !line.contains("\"session_meta\"") || !line.contains("\"model_provider\"") {
        return None;
    }
    let mut value: Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = value.get_mut("payload")?.as_object_mut()?;
    if payload.get("model_provider")?.as_str()? != CC_SWITCH_CODEX_MODEL_PROVIDER_ID {
        return None;
    }
    let session_id = payload.get("id")?.as_str()?;
    if !official_session_ids.contains(session_id) {
        return None;
    }
    payload.insert(
        "model_provider".to_string(),
        Value::String(OFFICIAL_OPENAI_CODEX_MODEL_PROVIDER_ID.to_string()),
    );
    serde_json::to_string(&value).ok()
}

fn rewrite_codex_session_file_lines(
    path: &Path,
    codex_dir: &Path,
    backup_root: &Path,
    rewrite_line: impl Fn(&str) -> Option<String>,
) -> Result<bool> {
    let metadata_before = fs::metadata(path)
        .map_err(|e| ApiError::Internal(format!("metadata {}: {e}", path.display())))?;
    let modified_before = metadata_before.modified().ok();
    let len_before = metadata_before.len();
    let content = fs::read_to_string(path)
        .map_err(|e| ApiError::Internal(format!("read {}: {e}", path.display())))?;

    let mut rewritten = String::with_capacity(content.len());
    let mut changed = false;
    for segment in content.split_inclusive('\n') {
        let (line, newline) = segment
            .strip_suffix('\n')
            .map(|line| (line, "\n"))
            .unwrap_or((segment, ""));
        if let Some(next_line) = rewrite_line(line) {
            rewritten.push_str(&next_line);
            changed = true;
        } else {
            rewritten.push_str(line);
        }
        rewritten.push_str(newline);
    }

    if !changed {
        return Ok(false);
    }

    ensure_codex_session_file_unchanged(path, modified_before, len_before)?;
    backup_codex_jsonl_file(path, codex_dir, backup_root)?;
    ensure_codex_session_file_unchanged(path, modified_before, len_before)?;
    atomic_write(path, rewritten.as_bytes())?;
    Ok(true)
}

fn ensure_codex_session_file_unchanged(
    path: &Path,
    modified_before: Option<SystemTime>,
    len_before: u64,
) -> Result<()> {
    let metadata_after = fs::metadata(path)
        .map_err(|e| ApiError::Internal(format!("metadata {}: {e}", path.display())))?;
    if metadata_after.modified().ok() != modified_before || metadata_after.len() != len_before {
        return Err(ApiError::Internal(format!(
            "Codex session file changed during restore: {}",
            path.display()
        )));
    }
    Ok(())
}

fn restore_codex_state_db_official_threads(
    db_path: &Path,
    codex_dir: &Path,
    official_thread_ids: &BTreeSet<String>,
    backup_root: &Path,
) -> Result<usize> {
    if !db_path.exists() || official_thread_ids.is_empty() {
        return Ok(0);
    }

    let mut conn = Connection::open(db_path)
        .map_err(|e| ApiError::Internal(format!("打开 Codex state DB 失败: {e}")))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| ApiError::Internal(format!("设置 Codex state DB busy_timeout 失败: {e}")))?;

    if !table_exists(&conn, "threads")? || !has_column(&conn, "threads", "model_provider")? {
        return Ok(0);
    }

    let ids: Vec<&String> = official_thread_ids.iter().collect();
    let mut matching_rows: i64 = 0;
    for chunk in ids.chunks(STATE_DB_ID_CHUNK) {
        let placeholders = placeholders(chunk.len());
        let count_sql = format!(
            "SELECT COUNT(*) FROM threads WHERE model_provider = ? AND id IN ({placeholders})"
        );
        let mut values: Vec<String> = Vec::with_capacity(chunk.len() + 1);
        values.push(CC_SWITCH_CODEX_MODEL_PROVIDER_ID.to_string());
        values.extend(chunk.iter().map(|id| (*id).clone()));
        let count: i64 = conn
            .query_row(
                &count_sql,
                rusqlite::params_from_iter(values.iter()),
                |row| row.get(0),
            )
            .map_err(|e| ApiError::Internal(format!("统计 Codex state DB 待还原行失败: {e}")))?;
        matching_rows += count;
    }
    if matching_rows == 0 {
        return Ok(0);
    }

    backup_codex_state_db(db_path, codex_dir, backup_root, &conn)?;

    let tx = conn
        .transaction()
        .map_err(|e| ApiError::Internal(format!("开启 Codex state DB 还原事务失败: {e}")))?;
    let mut changed = 0;
    for chunk in ids.chunks(STATE_DB_ID_CHUNK) {
        let placeholders = placeholders(chunk.len());
        let update_sql = format!(
            "UPDATE threads SET model_provider = ? WHERE model_provider = ? AND id IN ({placeholders})"
        );
        let mut values: Vec<String> = Vec::with_capacity(chunk.len() + 2);
        values.push(OFFICIAL_OPENAI_CODEX_MODEL_PROVIDER_ID.to_string());
        values.push(CC_SWITCH_CODEX_MODEL_PROVIDER_ID.to_string());
        values.extend(chunk.iter().map(|id| (*id).clone()));
        changed += tx
            .execute(&update_sql, rusqlite::params_from_iter(values.iter()))
            .map_err(|e| ApiError::Internal(format!("还原 Codex state DB provider 失败: {e}")))?;
    }
    tx.commit()
        .map_err(|e| ApiError::Internal(format!("提交 Codex state DB 还原事务失败: {e}")))?;
    Ok(changed)
}

fn collect_jsonl_files(dir: &Path, files: &mut Vec<PathBuf>, depth: u8, max_depth: u8) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files, depth + 1, max_depth);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

fn collect_files_with_extension(
    dir: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
    depth: u8,
    max_depth: u8,
) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_extension(&path, extension, files, depth + 1, max_depth);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            files.push(path);
        }
    }
}

fn codex_state_db_paths(codex_dir: &Path, config_text: &str) -> Vec<PathBuf> {
    let mut paths = vec![codex_dir.join(CODEX_STATE_DB_FILENAME)];
    if let Some(sqlite_home) = sqlite_home_from_codex_config(config_text) {
        let db_path = sqlite_home.join(CODEX_STATE_DB_FILENAME);
        if !paths.contains(&db_path) {
            paths.push(db_path);
        }
    }
    paths
}

fn sqlite_home_from_codex_config(config_text: &str) -> Option<PathBuf> {
    let doc = config_text.parse::<toml_edit::DocumentMut>().ok()?;
    let raw = doc.get("sqlite_home")?.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(resolve_user_path(raw))
}

fn resolve_user_path(raw: &str) -> PathBuf {
    if raw == "~" {
        return crate::state::home_dir();
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return crate::state::home_dir().join(rest);
    }
    if let Some(rest) = raw.strip_prefix("~\\") {
        return crate::state::home_dir().join(rest);
    }
    PathBuf::from(raw)
}

fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(", ")
}

fn backup_codex_jsonl_file(path: &Path, codex_dir: &Path, backup_root: &Path) -> Result<()> {
    let backup_path = backup_root
        .join("jsonl")
        .join(relative_backup_path(path, codex_dir));
    copy_existing_file(path, &backup_path)
}

fn backup_codex_state_db(
    db_path: &Path,
    codex_dir: &Path,
    backup_root: &Path,
    source_conn: &Connection,
) -> Result<()> {
    let backup_path = backup_root
        .join("state")
        .join(relative_backup_path(db_path, codex_dir));
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ApiError::Internal(format!("mkdir {}: {e}", parent.display())))?;
    }

    let mut backup_conn = Connection::open(&backup_path)
        .map_err(|e| ApiError::Internal(format!("创建 Codex state DB 备份失败: {e}")))?;
    let backup = Backup::new(source_conn, &mut backup_conn)
        .map_err(|e| ApiError::Internal(format!("初始化 Codex state DB 备份失败: {e}")))?;
    backup
        .run_to_completion(5, Duration::from_millis(25), None)
        .map_err(|e| ApiError::Internal(format!("写入 Codex state DB 备份失败: {e}")))?;
    Ok(())
}

fn copy_existing_file(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ApiError::Internal(format!("mkdir {}: {e}", parent.display())))?;
    }
    fs::copy(source, target).map_err(|e| {
        ApiError::Internal(format!(
            "copy {} -> {}: {e}",
            source.display(),
            target.display()
        ))
    })?;
    Ok(())
}

fn relative_backup_path(path: &Path, root: &Path) -> PathBuf {
    if let Ok(relative) = path.strip_prefix(root) {
        return relative.to_path_buf();
    }
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let hash = hasher.finish();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    PathBuf::from("external").join(format!("{hash:016x}-{file_name}"))
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let temp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&temp)
            .map_err(|e| ApiError::Internal(format!("create {}: {e}", temp.display())))?;
        file.write_all(data)
            .map_err(|e| ApiError::Internal(format!("write {}: {e}", temp.display())))?;
        file.sync_all()
            .map_err(|e| ApiError::Internal(format!("sync {}: {e}", temp.display())))?;
    }
    fs::rename(&temp, path).map_err(|e| {
        ApiError::Internal(format!(
            "rename {} -> {}: {e}",
            temp.display(),
            path.display()
        ))
    })?;
    Ok(())
}

fn read_codex_config_text() -> String {
    fs::read_to_string(codex_config_dir().join("config.toml")).unwrap_or_default()
}

fn codex_config_dir() -> PathBuf {
    crate::state::home_dir().join(".codex")
}

fn official_history_unify_backup_parent() -> PathBuf {
    crate::state::app_config_dir()
        .join("backups")
        .join(OFFICIAL_UNIFY_MIGRATION_NAME)
}

fn migration_backup_root(migration_name: &str) -> PathBuf {
    crate::state::app_config_dir()
        .join("backups")
        .join(migration_name)
        .join(chrono::Local::now().format("%Y%m%d_%H%M%S").to_string())
}

fn canonical_dir_string(dir: &Path) -> String {
    fs::canonicalize(dir)
        .unwrap_or_else(|_| dir.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|e| ApiError::Internal(format!("检查表存在失败: {e}")))?;
    Ok(count > 0)
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| ApiError::Internal(format!("获取表结构失败: {e}")))?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get("name")?;
        Ok(name)
    });
    if let Ok(rows) = rows {
        for name in rows.flatten() {
            if name == column {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_backup_only_for_current_codex_dir() {
        let dir = tempdir().expect("tempdir");
        let codex_dir = dir.path().join(".codex");
        let ledger_parent = dir.path().join("ledger");
        let codex_dir_key = canonical_dir_string(&codex_dir);

        assert!(!has_backup_for_dir(&ledger_parent, &codex_dir_key));

        let other = ledger_parent.join("20260612_010101");
        fs::create_dir_all(&other).expect("create other gen");
        fs::write(
            other.join("meta.json"),
            "{\n  \"codexConfigDir\": \"/some/other/codex-dir\"\n}",
        )
        .expect("write other meta");
        assert!(!has_backup_for_dir(&ledger_parent, &codex_dir_key));

        let matched = ledger_parent.join("20260612_020202");
        fs::create_dir_all(&matched).expect("create matched gen");
        fs::write(
            matched.join("meta.json"),
            format!("{{\n  \"codexConfigDir\": \"{codex_dir_key}\"\n}}"),
        )
        .expect("write matched meta");
        assert!(has_backup_for_dir(&ledger_parent, &codex_dir_key));
    }

    #[test]
    fn restore_only_ledgered_official_sessions() {
        let dir = tempdir().expect("tempdir");
        let codex_dir = dir.path().join(".codex");
        let ledger_parent = dir.path().join("ledger");
        let restore_backup_root = dir.path().join("restore-backup");

        // 备份账本：s1 / t1 属于官方 openai
        let generation = ledger_parent.join("20260612_010101");
        let backup_session_dir = generation.join("jsonl/sessions/2026/06/01");
        fs::create_dir_all(&backup_session_dir).expect("create backup session dir");
        fs::write(
            backup_session_dir.join("official.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"model_provider\":\"openai\"}}\n",
        )
        .expect("write backup session");
        let backup_state_dir = generation.join("state");
        fs::create_dir_all(&backup_state_dir).expect("create backup state dir");
        let backup_db = Connection::open(backup_state_dir.join(CODEX_STATE_DB_FILENAME))
            .expect("open backup db");
        backup_db
            .execute_batch(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT NOT NULL);
                INSERT INTO threads (id, model_provider) VALUES ('t1', 'openai');",
            )
            .expect("seed backup db");
        drop(backup_db);

        // 当前数据
        let session_dir = codex_dir.join("sessions/2026/06/01");
        fs::create_dir_all(&session_dir).expect("create session dir");
        let official_path = session_dir.join("official.jsonl");
        fs::write(
            &official_path,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"model_provider\":\"custom\"}}\n",
        )
        .expect("write official session");
        let on_period_path = codex_dir.join("sessions/2026/06/12/on-period.jsonl");
        fs::create_dir_all(on_period_path.parent().unwrap()).expect("create on-period dir");
        fs::write(
            &on_period_path,
            concat!(
                "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s2\",\"model_provider\":\"custom\"}}\n",
                "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s3\",\"model_provider\":\"my-private-relay\"}}\n",
            ),
        )
        .expect("write on-period session");

        let state_db_path = codex_dir.join(CODEX_STATE_DB_FILENAME);
        let conn = Connection::open(&state_db_path).expect("open state db");
        conn.execute_batch(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT NOT NULL);
            INSERT INTO threads (id, model_provider) VALUES
                ('t1', 'custom'),
                ('t2', 'custom'),
                ('t3', 'openai');",
        )
        .expect("seed state db");
        drop(conn);

        fs::write(
            generation.join("meta.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "codexConfigDir": canonical_dir_string(&codex_dir)
            }))
            .expect("serialize meta"),
        )
        .expect("write meta");

        let outcome =
            restore_official_history_inner(&codex_dir, &ledger_parent, &restore_backup_root, "")
                .expect("restore");

        assert_eq!(outcome.restored_jsonl_files, 1);
        assert_eq!(outcome.restored_state_rows, 1);
        assert!(outcome.skipped_reason.is_none());

        let official_text = fs::read_to_string(&official_path).expect("read official");
        assert!(official_text.contains("\"model_provider\":\"openai\""));
        let on_period_text = fs::read_to_string(&on_period_path).expect("read on-period");
        assert!(on_period_text.contains("\"id\":\"s2\",\"model_provider\":\"custom\""));
        assert!(on_period_text.contains("\"model_provider\":\"my-private-relay\""));

        let conn = Connection::open(&state_db_path).expect("reopen state db");
        let provider_of = |thread_id: &str| -> String {
            conn.query_row(
                "SELECT model_provider FROM threads WHERE id = ?1",
                [thread_id],
                |row| row.get(0),
            )
            .expect("thread provider")
        };
        assert_eq!(provider_of("t1"), "openai");
        assert_eq!(provider_of("t2"), "custom");
        assert_eq!(provider_of("t3"), "openai");
        drop(conn);

        assert!(restore_backup_root
            .join("jsonl/sessions/2026/06/01/official.jsonl")
            .exists());
        assert!(restore_backup_root
            .join("state")
            .join(CODEX_STATE_DB_FILENAME)
            .exists());

        // 幂等：第二次还原无事可做
        let rerun = restore_official_history_inner(
            &codex_dir,
            &ledger_parent,
            &dir.path().join("restore-backup-2"),
            "",
        )
        .expect("rerun restore");
        assert_eq!(rerun.restored_jsonl_files, 0);
        assert_eq!(rerun.restored_state_rows, 0);
        assert_eq!(rerun.skipped_reason.as_deref(), Some("nothing_to_restore"));
    }

    #[test]
    fn restore_skips_when_no_backup_ledger() {
        let dir = tempdir().expect("tempdir");
        let codex_dir = dir.path().join(".codex");
        let session_dir = codex_dir.join("sessions/2026/06/01");
        fs::create_dir_all(&session_dir).expect("create session dir");
        fs::write(
            session_dir.join("session.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"model_provider\":\"custom\"}}\n",
        )
        .expect("write session");

        let outcome = restore_official_history_inner(
            &codex_dir,
            &dir.path().join("missing-ledger"),
            &dir.path().join("restore-backup"),
            "",
        )
        .expect("restore");
        assert_eq!(outcome.skipped_reason.as_deref(), Some("no_backup_ledger"));
        assert_eq!(outcome.restored_jsonl_files, 0);
        assert_eq!(outcome.restored_state_rows, 0);
    }
}
