use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::Value;

use crate::AppState;

/// settings.json lives at the partitions root — it is global (not per-vault)
/// and is therefore never inside any vault's git repository.

pub async fn get_settings(State(state): State<Arc<AppState>>) -> Json<Value> {
    let path = state.root_path.join("settings.json");
    match tokio::fs::read_to_string(&path).await {
        Ok(s) => Json(serde_json::from_str(&s).unwrap_or(Value::Object(Default::default()))),
        Err(_) => Json(Value::Object(Default::default())),
    }
}

pub async fn put_settings(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> StatusCode {
    let path = state.root_path.join("settings.json");
    match serde_json::to_string_pretty(&body) {
        Ok(content) => match tokio::fs::write(&path, content).await {
            Ok(_) => StatusCode::NO_CONTENT,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        },
        Err(_) => StatusCode::BAD_REQUEST,
    }
}
