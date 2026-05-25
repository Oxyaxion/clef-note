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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Stable alphabetical sort first, then stable pinned-first
    notes.sort_by(|a, b| a.name.cmp(&b.name));
    notes.sort_by_key(|n| std::cmp::Reverse(n.pinned));

    Ok(Json(notes))
}

fn is_safe_note_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('\\')
        && !name.split('/').any(|seg| seg == "." || seg == "..")
}

fn note_path(storage: &std::path::Path, name: &str) -> std::path::PathBuf {
    storage.join("notes").join(format!("{name}.md"))
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
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let content = crate::frontmatter::stamp_last_modified(&body.content);
    tokio::fs::write(&path, &content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    tokio::task::spawn_blocking(move || db.upsert(&name_for_db, &parsed, mtime))
        .await
        .ok();

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
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tokio::fs::rename(&old_path, &new_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = state.db.clone();
    tokio::task::spawn_blocking({
        let name = name.clone();
        let new_name = new_name.clone();
        move || db.rename(&name, &new_name)
    })
    .await
    .ok();

    // Rewrite [[old_name]] wiki links in all other notes
    let notes_dir = state.storage_path.join("notes");
    let db_wl = state.db.clone();
    tokio::task::spawn_blocking({
        let notes_dir = notes_dir.clone();
        move || update_wiki_links_in_notes(&notes_dir, &name, &new_name, &db_wl)
    })
    .await
    .ok();

    let new_index = crate::backlinks::BacklinkIndex::build(&notes_dir).await;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = state.db.clone();
    let name_for_db = name.clone();
    tokio::task::spawn_blocking(move || db.delete(&name_for_db))
        .await
        .ok();

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
        let dest = state.storage_path.join("assets").join(&safe_name);
        tokio::fs::write(&dest, &data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let path = state.storage_path.join("assets").join(&safe_name);
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
    let dir = state.storage_path.join("assets");
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
    let path = state.storage_path.join("assets").join(sanitize_filename(&filename));
    let _ = tokio::fs::remove_file(path).await;
    StatusCode::NO_CONTENT
}

/// After renaming a note, rewrite all `[[old_name]]` / `[[old_name|alias]]`
/// occurrences in every other note to `[[new_name]]` / `[[new_name|alias]]`.
fn update_wiki_links_in_notes(
    notes_dir: &std::path::Path,
    old_name: &str,
    new_name: &str,
    db: &crate::db::Db,
) {
    let escaped = regex::escape(old_name);
    // Matches [[old_name]] and [[old_name|anything]]
    let Ok(re) = Regex::new(&format!(r"\[\[{}(\|[^\]]*)?]]", escaped)) else { return };

    for entry in walkdir::WalkDir::new(notes_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else { continue };

        // Skip files that can't possibly contain the link (fast path)
        if !content.contains(&format!("[[{}", old_name)) {
            continue;
        }

        let updated = re.replace_all(&content, |caps: &regex::Captures| {
            let alias = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            format!("[[{}{}]]", new_name, alias)
        }).into_owned();

        if updated == content {
            continue;
        }

        if std::fs::write(path, &updated).is_ok() {
            let Ok(rel) = path.strip_prefix(notes_dir) else { continue };
            let note_name = rel.with_extension("").to_string_lossy().replace('\\', "/");
            let parsed = crate::frontmatter::parse_note(&updated);
            db.upsert(&note_name, &parsed, read_mtime(path));
        }
    }
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
