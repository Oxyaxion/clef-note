mod auth;
mod backlinks;
mod config;
mod db;
mod drawings;
mod frontend;
mod frontmatter;
mod key;
mod notes;
mod oidc;
mod openapi;
mod partition;
mod query;
mod seed;
mod session;
mod settings;
mod shares;
mod sync;
mod vaults;

use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router, extract::{Request, State}, http::{StatusCode, header},
    middleware::{self, Next}, response::Response,
    routing::{delete, get, post},
};
use tower_http::cors::{Any, CorsLayer};

pub struct AppState {
    pub root_path: std::path::PathBuf,
    pub vaults: tokio::sync::RwLock<HashMap<String, Arc<vaults::VaultState>>>,
    pub active_vault: tokio::sync::RwLock<String>,
    pub password_hash: String,
    pub sessions: session::SessionStore,
    pub api_key: Option<String>,
    pub login_guard: auth::LoginGuard,
    pub oidc_client: Option<oidc::OidcClient>,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("clef-note {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Some(i) = args.iter().position(|a| a == "--hash-password") {
        match args.get(i + 1) {
            Some(pwd) => { println!("{}", auth::hash_password(pwd)); }
            None => { eprintln!("usage: clef-note --hash-password \"yourpassword\""); }
        }
        return;
    }

    tracing_subscriber::fmt::init();
    let (state, port) = setup_state().await;
    let state = Arc::new(state);
    start_sync_tasks(&state);
    start_share_purge_task(&state);
    run_server(state, port).await;
}

fn parse_arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let flag = format!("--{name}");
    for (i, arg) in args.iter().enumerate() {
        if let Some(val) = arg.strip_prefix(&format!("{flag}=")) {
            return Some(val.to_string());
        }
        if arg == &flag && let Some(val) = args.get(i + 1) {
            return Some(val.clone());
        }
    }
    None
}

async fn setup_state() -> (AppState, u16) {
    let default_partitions = std::path::PathBuf::from("../partitions");

    let cfg = config::load(&default_partitions);

    // Priority: --partitions arg > partitions in toml > ../partitions
    let root_path = parse_arg("partitions")
        .map(std::path::PathBuf::from)
        .or_else(|| cfg.partitions.as_deref().map(std::path::PathBuf::from))
        .unwrap_or(default_partitions);

    let port = parse_arg("port").and_then(|v| v.parse().ok()).or(cfg.port).unwrap_or(3000);

    tokio::fs::create_dir_all(&root_path).await.unwrap();

    // settings.json lives at the partitions root (global, never inside a vault)
    // No vault-specific init needed here.

    let vault_tokens = cfg.vault_tokens.clone().unwrap_or_default();
    let discovered = vaults::discover(&root_path, &vault_tokens).await;

    if discovered.is_empty() {
        // First run: auto-create a default "notes" partition.
        let vault = vaults::init(
            "notes".to_string(),
            "Notes".to_string(),
            root_path.join("notes"),
            None,
        ).await;
        let toml = "name = \"Notes\"\n";
        tokio::fs::write(root_path.join("notes").join("partition.toml"), toml).await.ok();
        let mut map: HashMap<String, Arc<vaults::VaultState>> = HashMap::new();
        map.insert("notes".to_string(), Arc::new(vault));
        let active = "notes".to_string();

        let oidc_client = init_oidc(&cfg).await;
        return (AppState {
            root_path,
            vaults: tokio::sync::RwLock::new(map),
            active_vault: tokio::sync::RwLock::new(active),
            password_hash: cfg.password.unwrap_or_default(),
            sessions: session::SessionStore::new(),
            api_key: cfg.api_key,
            login_guard: auth::LoginGuard::new(),
            oidc_client,
        }, port);
    }

    let active_slug = discovered[0].slug.clone();
    let mut map: HashMap<String, Arc<vaults::VaultState>> = HashMap::new();
    for v in discovered {
        map.insert(v.slug.clone(), v);
    }

    let oidc_client = init_oidc(&cfg).await;

    let state = AppState {
        root_path,
        vaults: tokio::sync::RwLock::new(map),
        active_vault: tokio::sync::RwLock::new(active_slug),
        password_hash: cfg.password.unwrap_or_default(),
        sessions: session::SessionStore::new(),
        api_key: cfg.api_key,
        login_guard: auth::LoginGuard::new(),
        oidc_client,
    };
    (state, port)
}

async fn init_oidc(cfg: &config::Config) -> Option<oidc::OidcClient> {
    let oidc_cfg = cfg.oidc.as_ref()?;
    match oidc::init(oidc_cfg).await {
        Ok(c) => {
            tracing::info!("OIDC configured with provider '{}'", c.provider_name);
            Some(c)
        }
        Err(e) => {
            eprintln!("error: OIDC init failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Start one periodic sync task per vault that has sync configured.
fn start_sync_tasks(state: &Arc<AppState>) {
    // Collect sync configs synchronously by blocking briefly — startup only.
    let vaults: Vec<(String, config::SyncConfig, std::path::PathBuf, sync::SharedSyncStatus)> = {
        // We're in a sync context here (called from main before the runtime is
        // fully yielded), so use try_read which is always available at startup.
        if let Ok(guard) = state.vaults.try_read() {
            guard.values()
                .filter_map(|v| {
                    v.sync_config.clone().map(|cfg| (
                        v.slug.clone(),
                        cfg,
                        v.storage_path.clone(),
                        v.sync_status.clone(),
                    ))
                })
                .collect()
        } else {
            vec![]
        }
    };

    for (slug, cfg, storage, status) in vaults {
        let slug_log = slug.clone();
        tokio::spawn(async move {
            tracing::info!("Starting sync for partition '{}'", slug_log);
            sync::run_sync(&cfg, &storage, &status).await;

            let interval_secs = cfg.interval_minutes.unwrap_or(0) * 60;
            if interval_secs > 0 {
                let mut ticker = tokio::time::interval(
                    tokio::time::Duration::from_secs(interval_secs),
                );
                ticker.tick().await;
                loop {
                    ticker.tick().await;
                    sync::run_sync(&cfg, &storage, &status).await;
                }
            }
        });
    }
}

fn start_share_purge_task(state: &Arc<AppState>) {
    let state = state.clone();
    tokio::spawn(async move {
        shares::purge_expired(&state).await;
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        ticker.tick().await;
        loop {
            ticker.tick().await;
            shares::purge_expired(&state).await;
        }
    });
}

// ── No-cache middleware ───────────────────────────────────────────────────────

async fn no_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("valid header value"),
    );
    response
}

// ── Sync API handlers (active vault) ─────────────────────────────────────────

async fn get_sync_status(
    State(_state): State<Arc<AppState>>,
    vaults::ActiveVault(vault): vaults::ActiveVault,
) -> Json<sync::SyncStatus> {
    Json(vault.sync_status.lock().unwrap().clone())
}

async fn trigger_sync(
    State(_state): State<Arc<AppState>>,
    vaults::ActiveVault(vault): vaults::ActiveVault,
) -> StatusCode {
    let Some(cfg) = vault.sync_config.clone() else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };
    let storage = vault.storage_path.clone();
    let status = vault.sync_status.clone();
    tokio::spawn(async move {
        sync::run_sync(&cfg, &storage, &status).await;
    });
    StatusCode::ACCEPTED
}

// ── Server ────────────────────────────────────────────────────────────────────

async fn run_server(state: Arc<AppState>, port: u16) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let protected = Router::new()
        .route("/notes", get(notes::list_notes))
        .route("/notes/{*name}", get(notes::get_note).put(notes::put_note).patch(notes::rename_note).delete(notes::delete_note))
        .route("/backlinks/{*name}", get(backlinks::get_backlinks))
        .route("/assets", post(notes::upload_asset))
        .route("/api/assets", get(notes::list_assets))
        .route("/assets/{*filename}", delete(notes::delete_asset))
        .route("/api/drawings", get(drawings::list_drawings))
        .route("/api/drawings/{*name}", get(drawings::get_drawing).put(drawings::put_drawing).delete(drawings::delete_drawing))
        .route("/api/drawing-preview/{*name}", get(drawings::get_preview).put(drawings::put_preview))
        .route("/api/query", get(query::handle_query))
        .route("/api/search", get(query::handle_search))
        .route("/api/tags", get(query::handle_tags))
        .route("/api/aliases", get(query::handle_aliases))
        .route("/api/field-values", get(query::handle_field_values))
        .route("/api/notes/stubs", get(notes::list_stubs))
        .route("/api/media-usage", get(notes::get_media_usage))
        .route("/api/key", get(key::get_keys))
        .route("/api/settings", get(settings::get_settings).put(settings::put_settings))
        .route("/api/openapi.json", get(openapi::get_spec))
        .route("/api/sync/status", get(get_sync_status))
        .route("/api/sync", post(trigger_sync))
        .route("/api/shares", get(shares::list_shares).post(shares::create_share))
        .route("/api/shares/{slug}", delete(shares::delete_share).patch(shares::update_share))
        .route("/api/vaults", get(vaults::list_vaults).post(vaults::create_vault))
        .route("/api/vaults/active", post(vaults::switch_vault))
        .route("/api/vaults/{slug}", delete(vaults::delete_vault))
        .route("/auth/logout", post(auth::logout))
        .layer(middleware::from_fn_with_state(state.clone(), auth::middleware))
        .layer(middleware::from_fn(no_cache));

    let app = Router::new()
        .merge(protected)
        .route("/auth/login", post(auth::login))
        .route("/auth/oidc/login", get(oidc::login_handler))
        .route("/auth/oidc/callback", get(oidc::callback_handler))
        .route("/auth/oidc/exchange", post(oidc::exchange_handler))
        .route("/api/auth/config", get(oidc::auth_config_handler))
        .route("/assets/{*filename}", get(notes::serve_asset))
        .route("/api/shared/{slug}", get(shares::get_shared))
        .fallback(frontend::handler)
        .with_state(state)
        .layer(cors);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on http://{addr}");
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.unwrap();
}
