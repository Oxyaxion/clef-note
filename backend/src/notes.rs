use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{db::NoteMeta, AppState};

/// Map an error to 500 while logging it — the real cause was previously
/// discarded by `.map_err(|_| INTERNAL_SERVER_ERROR)`.
fn to_500<E: std::fmt::Display>(e: E) -> StatusCode {
    tracing::error!("internal error: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

#[derive(Serialize)]
pub struct NoteContent {
    pub name: String,
    /// Body only — frontmatter stripped
    pub content: String,
    /// Parsed frontmatter as JSON object
    pub frontmatter: serde_json::Value,
}

#[derive(Deserialize)]
pub struct PutNoteBody {
    pub content: String,
}

#[derive(Serialize)]
pub struct MediaUsage {
    pub used_assets: Vec<String>,
    pub used_drawings: Vec<String>,
}

pub async fn get_media_usage(State(state): State<Arc<AppState>>) -> Json<MediaUsage> {
    let (assets_set, drawings_set) = state.db.get_media_usage();
    let mut used_assets: Vec<String> = assets_set.into_iter().collect();
    let mut used_drawings: Vec<String> = drawings_set.into_iter().collect();
    used_assets.sort();
    used_drawings.sort();
    Json(MediaUsage { used_assets, used_drawings })
}

#[derive(Deserialize)]
pub struct RenameBody {
    pub new_name: String,
}

pub async fn list_notes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<NoteMeta>>, StatusCode> {
    let db = state.db.clone();
    let mut notes: Vec<NoteMeta> = tokio::task::spawn_blocking(move || db.list_all_meta())
        .await
        .map_err(to_500)?;

    // Stable alphabetical sort first, then stable pinned-first
    notes.sort_by(|a, b| a.name.cmp(&b.name));
    notes.sort_by_key(|n| std::cmp::Reverse(n.pinned));

    Ok(Json(notes))
}

pub(crate) fn is_safe_note_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('\\')
        // Disallow `.`, `..`, and hidden segments (starting with `.`) to prevent
        // accessing `.assets/`, `.drawings/`, `.git/`, etc.
        && !name.split('/').any(|seg| seg == ".." || seg.starts_with('.'))
}

fn note_path(storage: &std::path::Path, name: &str) -> std::path::PathBuf {
    storage.join(format!("{name}.md"))
}

pub fn read_mtime(path: &std::path::Path) -> i64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub async fn get_note(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<NoteContent>, StatusCode> {
    if !is_safe_note_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = note_path(&state.storage_path, &name);
    let raw = tokio::fs::read_to_string(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let parsed = crate::frontmatter::parse_note(&raw);
    Ok(Json(NoteContent {
        name,
        content: parsed.body,
        frontmatter: parsed.frontmatter,
    }))
}

pub async fn put_note(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<PutNoteBody>,
) -> Result<StatusCode, StatusCode> {
    if !is_safe_note_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = note_path(&state.storage_path, &name);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(to_500)?;
    }
    let content = crate::frontmatter::stamp_last_modified(&body.content);
    tokio::fs::write(&path, &content)
        .await
        .map_err(to_500)?;

    // Update in-memory index (use mtime of the just-written file)
    let mtime = tokio::fs::metadata(&path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let parsed = crate::frontmatter::parse_note(&content);
    let db = state.db.clone();
    let name_for_db = name.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || db.upsert(&name_for_db, &parsed, mtime)).await {
        tracing::error!("db.upsert task failed for {name}: {e}");
    }

    // Incremental backlink update — no filesystem walk needed
    state.backlink_index.write().await.update_note(&name, &content);

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct AssetResponse {
    pub url: String,
}

pub async fn rename_note(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<RenameBody>,
) -> Result<StatusCode, StatusCode> {
    let new_name = body.new_name.trim().to_string();

    if !is_safe_note_name(&new_name) || new_name == name {
        return Err(StatusCode::BAD_REQUEST);
    }

    let old_path = note_path(&state.storage_path, &name);
    let new_path = note_path(&state.storage_path, &new_name);

    if !old_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    if new_path.exists() {
        return Err(StatusCode::CONFLICT);
    }

    if let Some(parent) = new_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(to_500)?;
    }

    tokio::fs::rename(&old_path, &new_path)
        .await
        .map_err(to_500)?;

    let db = state.db.clone();
    if let Err(e) = tokio::task::spawn_blocking({
        let name = name.clone();
        let new_name = new_name.clone();
        move || db.rename(&name, &new_name)
    })
    .await
    {
        tracing::error!("db.rename task failed for {name} → {new_name}: {e}");
    }

    // Rewrite [[old_name]] links AND rebuild the backlink index in one vault pass.
    let storage = state.storage_path.clone();
    let db_wl = state.db.clone();
    let new_index = tokio::task::spawn_blocking({
        let storage = storage.clone();
        let name = name.clone();
        let new_name = new_name.clone();
        move || rename_links_and_reindex(&storage, &name, &new_name, &db_wl)
    })
    .await
    .unwrap_or_default();

    *state.backlink_index.write().await = new_index;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_note(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<StatusCode, StatusCode> {
    if !is_safe_note_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = note_path(&state.storage_path, &name);
    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    tokio::fs::remove_file(&path)
        .await
        .map_err(to_500)?;

    let db = state.db.clone();
    let name_for_db = name.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || db.delete(&name_for_db)).await {
        tracing::error!("db.delete task failed for {name}: {e}");
    }

    state.backlink_index.write().await.remove_note(&name);

    Ok(StatusCode::NO_CONTENT)
}

pub async fn upload_asset(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<AssetResponse>, StatusCode> {
    if let Ok(Some(field)) = multipart.next_field().await {
        let filename = field.file_name().unwrap_or("asset").to_string();
        let safe_name = sanitize_filename(&filename);
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let dest = state.storage_path.join(".assets").join(&safe_name);
        tokio::fs::write(&dest, &data)
            .await
            .map_err(to_500)?;
        Ok(Json(AssetResponse {
            url: format!("/assets/{safe_name}"),
        }))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

pub async fn serve_asset(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let safe_name = sanitize_filename(&filename);
    let path = state.storage_path.join(".assets").join(&safe_name);
    let data = tokio::fs::read(&path).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let content_type = match safe_name.rsplit('.').next().unwrap_or("") {
        "png"  => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif"  => "image/gif",
        "webp" => "image/webp",
        "svg"  => "image/svg+xml",
        "avif" => "image/avif",
        _      => "application/octet-stream",
    };
    Ok(([(header::CONTENT_TYPE, content_type)], data).into_response())
}

#[derive(Serialize)]
pub struct AssetMeta {
    pub name: String,
    pub size: u64,
}

pub async fn list_assets(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let dir = state.storage_path.join(".assets");
    let assets: Vec<AssetMeta> = tokio::task::spawn_blocking(move || {
        let mut result = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() && let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    result.push(AssetMeta { name: name.to_string(), size });
                }
            }
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    })
    .await
    .unwrap_or_default();
    axum::Json(assets)
}

pub async fn delete_asset(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> StatusCode {
    let path = state.storage_path.join(".assets").join(sanitize_filename(&filename));
    let _ = tokio::fs::remove_file(path).await;
    StatusCode::NO_CONTENT
}

/// After renaming a note, rewrite all `[[old_name]]` / `[[old_name|alias]]`
/// occurrences in every other note to `[[new_name]]` / `[[new_name|alias]]`.
/// After renaming, rewrite `[[old_name]]` links in every note AND rebuild the
/// backlink index in the SAME filesystem pass (previously the vault was walked
/// twice: once here, once in `BacklinkIndex::build`). Returns the fresh index.
fn rename_links_and_reindex(
    notes_dir: &std::path::Path,
    old_name: &str,
    new_name: &str,
    db: &crate::db::Db,
) -> crate::backlinks::BacklinkIndex {
    let escaped = regex::escape(old_name);
    // Matches [[old_name]] and [[old_name|anything]]
    let re = Regex::new(&format!(r"\[\[{}(\|[^\]]*)?]]", escaped)).ok();
    let mut index = crate::backlinks::BacklinkIndex::default();

    for entry in walkdir::WalkDir::new(notes_dir)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_str().map_or(false, |s| s.starts_with('.')))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else { continue };
        let Ok(rel) = path.strip_prefix(notes_dir) else { continue };
        let note_name = rel.with_extension("").to_string_lossy().replace('\\', "/");

        // Rewrite links if this note references the old name (fast-path guard).
        let final_content = match &re {
            Some(re) if content.contains(&format!("[[{}", old_name)) => {
                let updated = re.replace_all(&content, |caps: &regex::Captures| {
                    let alias = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    format!("[[{}{}]]", new_name, alias)
                }).into_owned();
                if updated != content && std::fs::write(path, &updated).is_ok() {
                    let parsed = crate::frontmatter::parse_note(&updated);
                    db.upsert(&note_name, &parsed, read_mtime(path));
                    updated
                } else {
                    content
                }
            }
            _ => content,
        };

        // (Re)build this note's backlink entry from its final on-disk content.
        index.update_note(&note_name, &final_content);
    }

    index
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
