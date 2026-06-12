//! Model pricing.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ModelPricingInfo {
    pub model_id: String,
    pub display_name: String,
    pub input_cost_per_million: String,
    pub output_cost_per_million: String,
    pub cache_read_cost_per_million: String,
    pub cache_creation_cost_per_million: String,
}

pub async fn get_model_pricing(ctx: &Arc<AppContext>) -> Result<Value> {
    let conn = open_db(ctx)?;
    let mut stmt = conn
        .prepare(
            "SELECT model_id, display_name, input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
             FROM model_pricing ORDER BY display_name",
        )
        .map_err(|e| ApiError::Internal(format!("prepare get_model_pricing: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ModelPricingInfo {
                model_id: row.get(0)?,
                display_name: row.get(1)?,
                input_cost_per_million: row.get(2)?,
                output_cost_per_million: row.get(3)?,
                cache_read_cost_per_million: row.get(4)?,
                cache_creation_cost_per_million: row.get(5)?,
            })
        })
        .map_err(|e| ApiError::Internal(format!("query model_pricing: {e}")))?
        .filter_map(|r| r.ok())
        .map(|p| serde_json::to_value(&p).unwrap_or(Value::Null))
        .collect::<Vec<_>>();
    Ok(Value::Array(rows))
}

pub async fn update_model_pricing(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let model_id: String = require_arg(&args, "modelId")?;
    let display_name: String = require_arg(&args, "displayName")?;
    let input_cost: String = require_arg(&args, "inputCost")?;
    let output_cost: String = require_arg(&args, "outputCost")?;
    let cache_read_cost: String = require_arg(&args, "cacheReadCost")?;
    let cache_creation_cost: String = require_arg(&args, "cacheCreationCost")?;
    let conn = open_db(ctx)?;
    conn.execute(
        "INSERT OR REPLACE INTO model_pricing (
            model_id, display_name, input_cost_per_million, output_cost_per_million,
            cache_read_cost_per_million, cache_creation_cost_per_million
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            model_id,
            display_name,
            input_cost,
            output_cost,
            cache_read_cost,
            cache_creation_cost
        ],
    )
    .map_err(|e| ApiError::Internal(format!("update_model_pricing: {e}")))?;
    Ok(Value::Null)
}

pub async fn delete_model_pricing(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let model_id: String = require_arg(&args, "modelId")?;
    let conn = open_db(ctx)?;
    conn.execute("DELETE FROM model_pricing WHERE model_id = ?1", [model_id])
        .map_err(|e| ApiError::Internal(format!("delete_model_pricing: {e}")))?;
    Ok(Value::Null)
}

fn open_db(ctx: &Arc<AppContext>) -> Result<rusqlite::Connection> {
    let path = ctx.opts.data_dir.join(".cc-switch").join("cc-switch.db");
    rusqlite::Connection::open(&path)
        .map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}
