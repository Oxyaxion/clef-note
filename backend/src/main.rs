mod auth;
mod backlinks;
mod config;
mod db;
mod drawings;
mod frontend;
mod frontmatter;
mod key;
mod notes;
mod openapi;
mod query;
mod seed;
mod session;
mod settings;

use std::sync::Arc;

use axum::{Router, middleware, routing::{delete, get, post}};
use tower_http::cors::{Any, CorsLayer};

pub struct AppState {
    pub storage_path: std::path::PathBuf,
    pub backlink_index: tokio::sync::RwLock<backlinks::BacklinkIndex>,
    pub db: Arc<db::Db>,
    pub password_hash: String,
    pub sessions: session::SessionStore,
    pub api_key: Option<String>,
    pub login_guard: auth::LoginGuard,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --hash-password <plaintext> → print Argon2 hash and exit
    if let Some(i) = args.iter().position(|a| a == "--hash-password") {
        match args.get(i + 1) {
            Some(pwd) => { println!("{}", auth::hash_password(pwd)); }
            None => { eprintln!("usage: aura-notes --hash-password \"yourpassword\""); }
        }
        return;
    }

    tracing_subscriber::fmt::init();
    let (state, port) = setup_state().await;
    run_server(Arc::new(state), port).await;
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
    let default_storage = std::path::PathBuf::from("../storage");

    // Config file location is unchanged — always relative to the default storage dir.
    let cfg = config::load(&default_storage);

    // Priority: --storage arg > storage in toml > ../storage
    let storage_path = parse_arg("storage")
        .map(std::path::PathBuf::from)
        .or_else(|| cfg.storage.as_deref().map(std::path::PathBuf::from))
        .unwrap_or(default_storage);

    // Priority: --port arg > port in toml > 3000
    let port = parse_arg("port").and_then(|v| v.parse().ok()).or(cfg.port).unwrap_or(3000);

    tokio::fs::create_dir_all(storage_path.join("notes")).await.unwrap();
    tokio::fs::create_dir_all(storage_path.join("assets")).await.unwrap();
    tokio::fs::create_dir_all(storage_path.join("drawings")).await.unwrap();

    seed::seed_defaults(&storage_path.join("notes")).await;

    let db = Arc::new(db::Db::new());
    let backlink_index = backlinks::BacklinkIndex::build(&storage_path.join("notes")).await;

    let db_clone = db.clone();
    let notes_dir = storage_path.join("notes");
    tokio::task::spawn_blocking(move || index_all_notes(&db_clone, &notes_dir))
        .await
        .ok();

    let state = AppState {
        storage_path,
        backlink_index: tokio::sync::RwLock::new(backlink_index),
        db,
        password_hash: cfg.password,
        sessions: session::SessionStore::new(),
        api_key: cfg.api_key,
        login_guard: auth::LoginGuard::new(),
    };
    (state, port)
}

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
        .route("/api/media-usage", get(notes::get_media_usage))
        .route("/api/key", get(key::get_keys))
        .route("/api/settings", get(settings::get_settings).put(settings::put_settings))
        .route("/api/openapi.json", get(openapi::get_spec))
        .route("/auth/logout", post(auth::logout))
        .layer(middleware::from_fn_with_state(state.clone(), auth::middleware));

    // Public — no auth
    let app = Router::new()
        .merge(protected)
        .route("/auth/login", post(auth::login))
        .route("/assets/{*filename}", get(notes::serve_asset))
        .fallback(frontend::handler)
        .with_state(state)
        .layer(cors);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on http://{addr}");
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.unwrap();
}

fn index_all_notes(db: &db::Db, notes_dir: &std::path::Path) {
    let walker = walkdir::WalkDir::new(notes_dir);
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(notes_dir) else { continue };
        let name = rel.with_extension("").to_string_lossy().replace('\\', "/");
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = frontmatter::parse_note(&content);
            db.upsert(&name, &parsed, notes::read_mtime(path));
        }
    }
}
