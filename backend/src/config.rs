use std::fmt;
use serde::Deserialize;

const TEMPLATE: &str = r#"# Clef Note — configuration

# Password for the web UI login.
# Hash with: ./clef-note --hash-password "yourpassword"
password = ""

# Storage directory — optional. Defaults to ../storage (relative to the backend/).
# Can be overridden at runtime with: ./clef-note --storage /mnt/notes
# storage = "/mnt/notes"

# Port — optional. Defaults to 3000.
# Can be overridden at runtime with: ./clef-note --port 8080
# port = 3000

# API key — optional. For programmatic access (CLI, REST, OpenAI tools…).
# Generate with: openssl rand -hex 32
# api_key = ""

# Git sync — optional. Synchronise storage/ with a remote git repository.
# Supports GitHub, Gitea, Forgejo, GitLab (HTTPS + token).
# [sync]
# remote = "https://github.com/you/notes.git"
# branch = "main"
# token = "ghp_xxxx"          # GitHub PAT · Gitea/Forgejo token
# interval_minutes = 30       # 0 = manual only (use the Settings UI button)
# author_name = "clef-note"   # optional — commit author name
# author_email = "sync@local" # optional — commit author email
"#;

#[derive(Deserialize, Clone)]
pub struct SyncConfig {
    pub remote: String,
    pub branch: String,
    pub token: String,
    pub interval_minutes: Option<u64>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

// Never print the token in logs.
impl fmt::Debug for SyncConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncConfig")
            .field("remote", &self.remote)
            .field("branch", &self.branch)
            .field("token", &"[REDACTED]")
            .field("interval_minutes", &self.interval_minutes)
            .finish()
    }
}

#[derive(Deserialize)]
pub struct Config {
    pub password: String,
    pub storage: Option<String>,
    pub port: Option<u16>,
    pub api_key: Option<String>,
    pub sync: Option<SyncConfig>,
}

pub fn resolve_path(storage_path: &std::path::Path) -> std::path::PathBuf {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if let Some(val) = arg.strip_prefix("--config=") {
            return std::path::PathBuf::from(val);
        }
        if arg == "--config" && let Some(p) = args.get(i + 1) {
            return std::path::PathBuf::from(p);
        }
    }
    if let Ok(p) = std::env::var("AURA_NOTES_CONFIG") && !p.is_empty() {
        return std::path::PathBuf::from(p);
    }
    storage_path.parent().unwrap_or(storage_path).join("clef-note.toml")
}

pub fn load(storage_path: &std::path::Path) -> Config {
    let path = resolve_path(storage_path);

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            std::fs::write(&path, TEMPLATE).ok();
            eprintln!("error: clef-note.toml not found — a template has been created at {}", path.display());
            eprintln!("       Set password (./clef-note --hash-password \"yourpassword\") and restart.");
            std::process::exit(1);
        }
    };

    let cfg: Config = match toml::from_str(&raw) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to parse clef-note.toml: {e}");
            std::process::exit(1);
        }
    };

    if cfg.password.trim().is_empty() {
        eprintln!("error: password is not set in clef-note.toml");
        eprintln!("       Hash one with: ./clef-note --hash-password \"yourpassword\"");
        std::process::exit(1);
    }

    if !cfg.password.starts_with("$argon2") {
        eprintln!("error: password in clef-note.toml must be an Argon2 hash");
        eprintln!("       Generate with: ./clef-note --hash-password \"yourpassword\"");
        std::process::exit(1);
    }

    if let Some(sync) = &cfg.sync {
        if sync.remote.trim().is_empty() || sync.token.trim().is_empty() || sync.branch.trim().is_empty() {
            eprintln!("error: [sync] requires remote, branch, and token to be set");
            std::process::exit(1);
        }
    }

    cfg
}
