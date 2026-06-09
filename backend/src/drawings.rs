use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
};

use crate::{AppState, notes::is_safe_note_name, vaults::ActiveVault};

pub async fn list_drawings(
    State(_state): State<Arc<AppState>>,
    ActiveVault(vault): ActiveVault,
) -> impl IntoResponse {
    let dir = vault.storage_path.join(".drawings");
    let names: Vec<String> = tokio::task::spawn_blocking(move || {
        let mut result = Vec::new();
        if let Ok(walker) = std::fs::read_dir(&dir) {
            for entry in walker.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("excalidraw")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    result.push(stem.to_string());
                }
            }
        }
        result.sort();
        result
    })
    .await
    .unwrap_or_default();

    axum::Json(names)
}

pub async fn get_drawing(
    State(_state): State<Arc<AppState>>,
    ActiveVault(vault): ActiveVault,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if !is_safe_note_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = vault.storage_path.join(".drawings").join(format!("{name}.excalidraw"));
    let content = tokio::fs::read(&path).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(([(header::CONTENT_TYPE, "application/json")], content).into_response())
}

pub async fn put_drawing(
    State(_state): State<Arc<AppState>>,
    ActiveVault(vault): ActiveVault,
    Path(name): Path<String>,
    body: axum::body::Bytes,
) -> Result<StatusCode, StatusCode> {
    if !is_safe_note_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = vault.storage_path.join(".drawings").join(format!("{name}.excalidraw"));
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    tokio::fs::write(&path, body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_drawing(
    State(_state): State<Arc<AppState>>,
    ActiveVault(vault): ActiveVault,
    Path(name): Path<String>,
) -> StatusCode {
    if !is_safe_note_name(&name) {
        return StatusCode::BAD_REQUEST;
    }
    let drawings_dir = vault.storage_path.join(".drawings");
    let _ = tokio::fs::remove_file(drawings_dir.join(format!("{name}.excalidraw"))).await;
    let _ = tokio::fs::remove_file(drawings_dir.join(format!("{name}.svg"))).await;
    StatusCode::NO_CONTENT
}

pub async fn get_preview(
    State(_state): State<Arc<AppState>>,
    ActiveVault(vault): ActiveVault,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if !is_safe_note_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = vault.storage_path.join(".drawings").join(format!("{name}.svg"));
    let content = tokio::fs::read(&path).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(([(header::CONTENT_TYPE, "image/svg+xml")], content).into_response())
}

pub async fn put_preview(
    State(_state): State<Arc<AppState>>,
    ActiveVault(vault): ActiveVault,
    Path(name): Path<String>,
    body: axum::body::Bytes,
) -> Result<StatusCode, StatusCode> {
    if !is_safe_note_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = vault.storage_path.join(".drawings").join(format!("{name}.svg"));
    tokio::fs::write(&path, body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
