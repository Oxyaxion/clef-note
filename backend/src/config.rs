use std::fmt;
use serde::Deserialize;

const TEMPLATE: &str = r#"# Clef Note — configuration

# Password for the web UI login.
# Hash with: ./clef-note --hash-password "yourpassword"
# Not required when [oidc] is configured.
password = ""

# Partitions root directory — optional. Defaults to ../partitions (relative to the binary).
# Can be overridden at runtime with: ./clef-note --partitions /mnt/notes
# partitions = "/mnt/notes"

# Port — optional. Defaults to 3000.
# Can be overridden at runtime with: ./clef-note --port 8080
# port = 3000

# API key — optional. For programmatic access (CLI, REST, OpenAI tools…).
# Generate with: openssl rand -hex 32
# api_key = ""

# Git sync tokens — one entry per partition slug.
# The token is kept here (outside any partition directory) so it is never
# accidentally committed to a git repository.
# [partition_tokens]
# notes = "ghp_xxxx"
# work  = "ghp_yyyy"

# OIDC — optional. Delegate authentication to an external provider.
# Works with Authelia, Authentik, Keycloak, and any OIDC-compliant provider.
# When configured, password login is disabled entirely.
# [oidc]
# issuer_url    = "https://auth.example.com"   # provider discovery URL
# client_id     = "clef-note"
# client_secret = "..."
# redirect_uri  = "https://notes.example.com/auth/oidc/callback"
# allowed_email = "user@example.com"           # restrict to a single user
# provider_name = "Authelia"                   # label shown on the login button (optional)
# disable_password_login = true                # hide password form when OIDC is active (optional)
"#;

#[derive(Deserialize, Clone)]
pub struct OidcConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub allowed_email: Option<String>,
    pub provider_name: Option<String>,
    pub disable_password_login: Option<bool>,
}

// Never print the secret in logs.
impl fmt::Debug for OidcConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OidcConfig")
            .field("issuer_url", &self.issuer_url)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("redirect_uri", &self.redirect_uri)
            .field("allowed_email", &self.allowed_email)
            .field("provider_name", &self.provider_name)
            .finish()
    }
}

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
    pub password: Option<String>,
    pub partitions: Option<String>,
    pub port: Option<u16>,
    pub api_key: Option<String>,
    pub partition_tokens: Option<std::collections::HashMap<String, String>>,
    pub sync: Option<SyncConfig>,
    pub oidc: Option<OidcConfig>,
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

    if cfg.oidc.is_none() {
        let pwd = cfg.password.as_deref().unwrap_or("").trim();
        if pwd.is_empty() {
            eprintln!("error: password is not set in clef-note.toml");
            eprintln!("       Hash one with: ./clef-note --hash-password \"yourpassword\"");
            eprintln!("       Or configure [oidc] for SSO authentication.");
            std::process::exit(1);
        }
        if !pwd.starts_with("$argon2") {
            eprintln!("error: password in clef-note.toml must be an Argon2 hash");
            eprintln!("       Generate with: ./clef-note --hash-password \"yourpassword\"");
            std::process::exit(1);
        }
    }

    if let Some(oidc) = &cfg.oidc {
        if oidc.issuer_url.trim().is_empty() || oidc.client_id.trim().is_empty()
            || oidc.client_secret.trim().is_empty() || oidc.redirect_uri.trim().is_empty()
        {
            eprintln!("error: [oidc] requires issuer_url, client_id, client_secret, and redirect_uri");
            std::process::exit(1);
        }
    }

    if let Some(sync) = &cfg.sync {
        if sync.remote.trim().is_empty() || sync.token.trim().is_empty() || sync.branch.trim().is_empty() {
            eprintln!("error: [sync] requires remote, branch, and token to be set");
            std::process::exit(1);
        }
    }

    cfg
}
