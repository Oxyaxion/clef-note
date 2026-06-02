use std::{
    collections::HashMap,
    net::IpAddr,
    sync::RwLock,
    time::{Duration, Instant},
};
use std::sync::Arc;

use argon2::{Argon2, PasswordHasher, PasswordVerifier, password_hash::{PasswordHash, SaltString}};
use axum::{Json, extract::{ConnectInfo, Request, State}, http::{HeaderMap, StatusCode}, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};

use crate::AppState;

// ── Brute-force guard ─────────────────────────────────────────────────────────

const MAX_FAILURES: u32 = 3;
const LOCKOUT: Duration = Duration::from_secs(60);

struct FailRecord {
    count: u32,
    locked_until: Option<Instant>,
}

pub struct LoginGuard {
    map: RwLock<HashMap<IpAddr, FailRecord>>,
}

impl LoginGuard {
    pub fn new() -> Self {
        Self { map: RwLock::new(HashMap::new()) }
    }

    pub fn is_locked(&self, ip: IpAddr) -> bool {
        let map = self.map.read().unwrap();
        map.get(&ip)
            .and_then(|r| r.locked_until)
            .is_some_and(|until| Instant::now() < until)
    }

    pub fn record_failure(&self, ip: IpAddr) {
        let mut map = self.map.write().unwrap();
        let rec = map.entry(ip).or_insert(FailRecord { count: 0, locked_until: None });
        // Reset if a previous lockout has expired
        if rec.locked_until.is_some_and(|u| Instant::now() >= u) {
            rec.count = 0;
            rec.locked_until = None;
        }
        rec.count += 1;
        if rec.count >= MAX_FAILURES {
            rec.locked_until = Some(Instant::now() + LOCKOUT);
        }
    }

    pub fn record_success(&self, ip: IpAddr) {
        self.map.write().unwrap().remove(&ip);
    }
}

// ── Middleware ────────────────────────────────────────────────────────────────

pub async fn middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let ok = state.sessions.is_valid(&token)
        || state.api_key.as_deref() == Some(token.as_str());

    if ok { Ok(next.run(req).await) } else { Err(StatusCode::UNAUTHORIZED) }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginPayload {
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<LoginResponse>, StatusCode> {
    if state.oidc_client.as_ref().map_or(false, |c| c.disable_password_login) {
        return Err(StatusCode::FORBIDDEN);
    }

    let ip = addr.ip();

    if state.login_guard.is_locked(ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let hash = PasswordHash::new(&state.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if Argon2::default()
        .verify_password(payload.password.as_bytes(), &hash)
        .is_err()
    {
        state.login_guard.record_failure(ip);
        return Err(StatusCode::UNAUTHORIZED);
    }

    state.login_guard.record_success(ip);
    Ok(Json(LoginResponse { token: state.sessions.create() }))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> StatusCode {
    if let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        state.sessions.revoke(token);
    }
    StatusCode::NO_CONTENT
}

// ── Password hashing (used by --hash-password CLI flag) ───────────────────────

pub fn hash_password(password: &str) -> String {
    use rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash failed")
        .to_string()
}
