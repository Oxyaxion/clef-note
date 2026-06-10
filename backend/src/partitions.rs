use std::{collections::HashMap, path::PathBuf, sync::Arc};

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::{AppState, config::SyncConfig, db::Db, sync};

// ── PartitionState ────────────────────────────────────────────────────────────

pub struct PartitionState {
    pub slug: String,
    pub name: std::sync::RwLock<String>,
    pub storage_path: PathBuf,
    pub db: Arc<Db>,
    pub backlink_index: tokio::sync::RwLock<crate::backlinks::BacklinkIndex>,
    pub sync_config: Option<SyncConfig>,
    pub sync_status: sync::SharedSyncStatus,
}

// ── API types ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct PartitionInfo {
    pub slug: String,
    pub name: String,
    pub active: bool,
    pub has_sync: bool,
}

#[derive(Deserialize)]
pub struct SwitchRequest {
    pub slug: String,
}

#[derive(Deserialize)]
pub struct CreateRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct RenameRequest {
    pub name: String,
}

// ── Discovery ─────────────────────────────────────────────────────────────────

/// Read the root `partition.toml` manifest and build one `PartitionState` per
/// declared partition (slug = sub-directory name). Sub-directories not listed
/// in the manifest are ignored. Returns an ordered list (alphabetical by slug).
pub async fn discover(
    root: &std::path::Path,
    partition_tokens: &HashMap<String, String>,
) -> Vec<Arc<PartitionState>> {
    crate::partition::migrate_legacy(root);

    let mut entries: Vec<(String, crate::partition::PartitionConfig)> =
        crate::partition::load_manifest(root)
            .into_iter()
            .filter(|(slug, _)| !slug.is_empty() && !slug.starts_with('.'))
            .collect();

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut partitions = Vec::new();
    for (slug, cfg) in entries {
        let path = root.join(&slug);
        let name = cfg.name.unwrap_or_else(|| slug.clone());
        let mut sync_cfg = cfg.sync;
        if let Some(ref mut sync) = sync_cfg {
            if let Some(token) = partition_tokens.get(&slug) {
                sync.token = Some(token.clone());
            }
        }
        let partition = init(slug, name, path, sync_cfg).await;
        partitions.push(Arc::new(partition));
    }

    partitions
}

/// Initialise a single partition: create dirs, seed defaults, build index.
pub async fn init(
    slug: String,
    name: String,
    storage_path: PathBuf,
    sync_config: Option<SyncConfig>,
) -> PartitionState {
    tokio::fs::create_dir_all(storage_path.join(".assets")).await.ok();
    tokio::fs::create_dir_all(storage_path.join(".drawings")).await.ok();

    crate::seed::seed_defaults(&storage_path).await;

    let db = Arc::new(Db::new());
    let backlink_index = crate::backlinks::BacklinkIndex::build(&storage_path).await;

    let db_clone = db.clone();
    let notes_dir = storage_path.clone();
    tokio::task::spawn_blocking(move || crate::db::index_dir(&db_clone, &notes_dir))
        .await
        .ok();

    let sync_status = sync::new_status(sync_config.is_some());

    PartitionState {
        slug,
        name: std::sync::RwLock::new(name),
        storage_path,
        db,
        backlink_index: tokio::sync::RwLock::new(backlink_index),
        sync_config,
        sync_status,
    }
}

// ── Axum extractor ────────────────────────────────────────────────────────────

/// Injects the currently active `PartitionState` into a handler.
pub struct ActivePartition(pub Arc<PartitionState>);

impl axum::extract::FromRequestParts<Arc<AppState>> for ActivePartition {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let slug = state.active_partition.read().await.clone();
        let partitions = state.partitions.read().await;
        let partition = partitions
            .get(&slug)
            .cloned()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "active partition not found"))?;
        Ok(ActivePartition(partition))
    }
}

// ── API handlers ──────────────────────────────────────────────────────────────

pub async fn list_partitions(State(state): State<Arc<AppState>>) -> Json<Vec<PartitionInfo>> {
    let active = state.active_partition.read().await.clone();
    let partitions = state.partitions.read().await;
    let mut infos: Vec<PartitionInfo> = partitions
        .values()
        .map(|p| PartitionInfo {
            slug: p.slug.clone(),
            name: p.name.read().unwrap().clone(),
            active: p.slug == active,
            has_sync: p.sync_config.is_some(),
        })
        .collect();
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    Json(infos)
}

pub async fn switch_partition(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SwitchRequest>,
) -> StatusCode {
    let exists = state.partitions.read().await.contains_key(&body.slug);
    if !exists {
        return StatusCode::NOT_FOUND;
    }
    *state.active_partition.write().await = body.slug;
    StatusCode::NO_CONTENT
}

pub async fn create_partition(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateRequest>,
) -> Result<Json<PartitionInfo>, StatusCode> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let slug: String = name
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();
    if slug.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    if state.partitions.read().await.contains_key(&slug) {
        return Err(StatusCode::CONFLICT);
    }

    let partition_dir = state.root_path.join(&slug);
    tokio::fs::create_dir_all(&partition_dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Register the partition in the root manifest.
    let manifest_path = crate::partition::manifest_path(&state.root_path);
    let current = tokio::fs::read_to_string(&manifest_path).await.unwrap_or_default();
    let updated = crate::partition::set_name(&current, &slug, &name);
    tokio::fs::write(&manifest_path, updated)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let partition = init(slug.clone(), name.clone(), partition_dir, None).await;
    let info = PartitionInfo { slug: slug.clone(), name, active: false, has_sync: false };
    state.partitions.write().await.insert(slug, Arc::new(partition));

    Ok(Json(info))
}

pub async fn rename_partition(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Json(body): Json<RenameRequest>,
) -> StatusCode {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return StatusCode::BAD_REQUEST;
    }

    let partition = {
        let partitions = state.partitions.read().await;
        match partitions.get(&slug) {
            Some(p) => p.clone(),
            None => return StatusCode::NOT_FOUND,
        }
    };

    // Update the partition's name in the root manifest, preserving its
    // [slug.sync] section and any surrounding comments.
    let manifest_path = crate::partition::manifest_path(&state.root_path);
    let current = tokio::fs::read_to_string(&manifest_path).await.unwrap_or_default();
    let updated = crate::partition::set_name(&current, &slug, &name);
    if tokio::fs::write(&manifest_path, updated).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    *partition.name.write().unwrap() = name;
    StatusCode::NO_CONTENT
}

pub async fn delete_partition(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> StatusCode {
    let active = state.active_partition.read().await.clone();
    if active == slug {
        return StatusCode::CONFLICT;
    }

    let partition_dir = {
        let partitions = state.partitions.read().await;
        match partitions.get(&slug) {
            Some(p) => p.storage_path.clone(),
            None => return StatusCode::NOT_FOUND,
        }
    };

    state.partitions.write().await.remove(&slug);

    // Drop the partition from the root manifest so it is not rediscovered.
    let manifest_path = crate::partition::manifest_path(&state.root_path);
    if let Ok(current) = tokio::fs::read_to_string(&manifest_path).await {
        let updated = crate::partition::remove(&current, &slug);
        tokio::fs::write(&manifest_path, updated).await.ok();
    }

    tokio::fs::remove_dir_all(&partition_dir).await.ok();

    StatusCode::NO_CONTENT
}
