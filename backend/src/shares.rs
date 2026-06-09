use std::collections::HashMap;
use std::sync::Arc;

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use rand_core::OsRng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::{AppState, notes::is_safe_note_name, partitions::ActivePartition};

static DRAWING_RE: OnceLock<Regex> = OnceLock::new();
static QUERY_RE: OnceLock<Regex> = OnceLock::new();
static EMBED_RE: OnceLock<Regex> = OnceLock::new();
static WIKI_ALIAS_RE: OnceLock<Regex> = OnceLock::new();
static WIKI_RE: OnceLock<Regex> = OnceLock::new();
static MULTI_BLANK_RE: OnceLock<Regex> = OnceLock::new();

// ── Data model ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub slug: String,
    pub note: String,
    /// Partition slug the note belongs to. None for legacy shares (fallback to first partition).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
}

/// .shares.json lives at the partitions root — global across all partitions so
/// that public URLs stay stable when the user switches the active partition.
fn shares_path(state: &AppState) -> std::path::PathBuf {
    state.root_path.join(".shares.json")
}

async fn load_shares(state: &AppState) -> HashMap<String, Share> {
    match tokio::fs::read_to_string(shares_path(state)).await {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

async fn save_shares(state: &AppState, shares: &HashMap<String, Share>) -> bool {
    match serde_json::to_string_pretty(shares) {
        Ok(json) => tokio::fs::write(shares_path(state), json).await.is_ok(),
        Err(_) => false,
    }
}

fn share_view(share: &Share) -> serde_json::Value {
    serde_json::json!({
        "slug":         share.slug,
        "note":         share.note,
        "created_at":   share.created_at,
        "expires_at":   share.expires_at,
        "has_password": share.password_hash.is_some(),
    })
}

fn is_valid_slug(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

// ── Preprocessing ─────────────────────────────────────────────────────────

pub fn preprocess_for_share(content: &str) -> String {
    let drawing_re = DRAWING_RE.get_or_init(|| Regex::new(r"(?s)```drawing\n.*?```").unwrap());
    let query_re   = QUERY_RE.get_or_init(||   Regex::new(r"(?s)```query\n.*?```").unwrap());
    let embed_re   = EMBED_RE.get_or_init(||   Regex::new(r"!\[\[[^\]]+\]\]").unwrap());
    let alias_re   = WIKI_ALIAS_RE.get_or_init(|| Regex::new(r"\[\[[^\]|]+\|([^\]]+)\]\]").unwrap());
    let wiki_re    = WIKI_RE.get_or_init(||    Regex::new(r"\[\[([^\]|]+)\]\]").unwrap());
    let blank_re   = MULTI_BLANK_RE.get_or_init(|| Regex::new(r"\n{3,}").unwrap());

    let s = drawing_re.replace_all(content, "");
    let s = query_re.replace_all(&s, "");
    let s = embed_re.replace_all(&s, "");
    let s = alias_re.replace_all(&s, "$1");
    let s = wiki_re.replace_all(&s, "$1");
    blank_re.replace_all(&s, "\n\n").trim().to_string()
}

pub async fn purge_expired(state: &AppState) {
    let mut shares = load_shares(state).await;
    let before = shares.len();
    shares.retain(|_, s| s.expires_at.map_or(true, |exp| Utc::now() <= exp));
    if shares.len() < before {
        save_shares(state, &shares).await;
        tracing::info!("Purged {} expired share(s)", before - shares.len());
    }
}

// ── Authenticated CRUD ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateShareRequest {
    pub slug: String,
    pub note: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub password: Option<String>,
}

pub async fn create_share(
    State(state): State<Arc<AppState>>,
    ActivePartition(partition): ActivePartition,
    Json(body): Json<CreateShareRequest>,
) -> impl IntoResponse {
    if !is_valid_slug(&body.slug) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid slug"}))).into_response();
    }
    if !is_safe_note_name(&body.note) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid note name"}))).into_response();
    }

    let mut shares = load_shares(&state).await;
    if shares.contains_key(&body.slug) {
        return (StatusCode::CONFLICT, Json(serde_json::json!({"error": "Slug already in use"}))).into_response();
    }

    let password_hash = match body.password.as_deref().filter(|p| !p.is_empty()) {
        Some(pw) => {
            let salt = SaltString::generate(&mut OsRng);
            match Argon2::default().hash_password(pw.as_bytes(), &salt) {
                Ok(h) => Some(h.to_string()),
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        None => None,
    };

    let share = Share {
        slug: body.slug.clone(),
        note: body.note,
        partition: Some(partition.slug.clone()),
        created_at: Utc::now(),
        expires_at: body.expires_at,
        password_hash,
    };
    let resp = share_view(&share);
    shares.insert(body.slug, share);

    if !save_shares(&state, &shares).await {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    (StatusCode::CREATED, Json(resp)).into_response()
}

pub async fn list_shares(
    State(state): State<Arc<AppState>>,
    ActivePartition(partition): ActivePartition,
) -> Json<serde_json::Value> {
    let mut list: Vec<serde_json::Value> = load_shares(&state)
        .await
        .values()
        .filter(|s| s.partition.as_deref() == Some(&partition.slug) || s.partition.is_none())
        .map(share_view)
        .collect();
    list.sort_by(|a, b| {
        b["created_at"].as_str().unwrap_or("").cmp(a["created_at"].as_str().unwrap_or(""))
    });
    Json(serde_json::json!(list))
}

pub async fn delete_share(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> StatusCode {
    let mut shares = load_shares(&state).await;
    if shares.remove(&slug).is_none() {
        return StatusCode::NOT_FOUND;
    }
    if save_shares(&state, &shares).await { StatusCode::NO_CONTENT } else { StatusCode::INTERNAL_SERVER_ERROR }
}

#[derive(Deserialize)]
pub struct UpdateShareRequest {
    pub expires_at: Option<serde_json::Value>,
    pub password: Option<String>,
}

pub async fn update_share(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(body): Json<UpdateShareRequest>,
) -> impl IntoResponse {
    let mut shares = load_shares(&state).await;
    let Some(share) = shares.get_mut(&slug) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if let Some(exp) = &body.expires_at {
        share.expires_at = if exp.is_null() {
            None
        } else {
            serde_json::from_value(exp.clone()).ok()
        };
    }

    if let Some(pw) = body.password.as_deref() {
        if pw.is_empty() {
            share.password_hash = None;
        } else {
            let salt = SaltString::generate(&mut OsRng);
            match Argon2::default().hash_password(pw.as_bytes(), &salt) {
                Ok(h) => share.password_hash = Some(h.to_string()),
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }

    let resp = share_view(share);
    if !save_shares(&state, &shares).await {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(resp).into_response()
}

// ── Public endpoint (no auth) ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SharedQuery {
    pub raw: Option<String>,
}

pub async fn get_shared(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Path(slug): Path<String>,
    Query(params): Query<SharedQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let ip = addr.ip();
    let shares = load_shares(&state).await;
    let Some(share) = shares.get(&slug) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if let Some(exp) = share.expires_at {
        if Utc::now() > exp {
            let mut shares = shares;
            shares.remove(&slug);
            save_shares(&state, &shares).await;
            return StatusCode::GONE.into_response();
        }
    }

    if let Some(hash_str) = &share.password_hash {
        if state.login_guard.is_locked(ip) {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        }

        let provided = headers.get("x-share-password").and_then(|v| v.to_str().ok());
        let ok = provided
            .and_then(|pw| PasswordHash::new(hash_str).ok().map(|h| (pw, h)))
            .map(|(pw, h)| Argon2::default().verify_password(pw.as_bytes(), &h).is_ok())
            .unwrap_or(false);

        if !ok {
            state.login_guard.record_failure(ip);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Password required", "password_required": true})),
            ).into_response();
        }
        state.login_guard.record_success(ip);
    }

    // Resolve partition for this share (fall back to first partition for legacy shares)
    let partition_slug = share.partition.clone();
    let storage_path = {
        let partitions = state.partitions.read().await;
        let partition = match &partition_slug {
            Some(slug) => partitions.get(slug.as_str()),
            None => partitions.values().next(),
        };
        match partition {
            Some(p) => p.storage_path.clone(),
            None => return StatusCode::NOT_FOUND.into_response(),
        }
    };

    let note_path = storage_path.join(format!("{}.md", share.note.replace('/', std::path::MAIN_SEPARATOR_STR)));
    let raw_file = match tokio::fs::read_to_string(&note_path).await {
        Ok(c) => c,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let parsed = crate::frontmatter::parse_note(&raw_file);
    let content = preprocess_for_share(&parsed.body);
    let title = parsed.title.unwrap_or_else(|| {
        share.note.split('/').next_back().unwrap_or(&share.note).to_string()
    });

    if params.raw.is_some() {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("# {title}\n\n{content}"),
        ).into_response();
    }

    Json(serde_json::json!({
        "slug":       share.slug,
        "title":      title,
        "content":    content,
        "note":       share.note,
        "expires_at": share.expires_at,
    })).into_response()
}
