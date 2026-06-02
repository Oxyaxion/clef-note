use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

const PENDING_TTL: Duration = Duration::from_secs(600);
const CODE_TTL: Duration = Duration::from_secs(60);

struct PendingState {
    pkce_verifier: PkceCodeVerifier,
    nonce: Nonce,
    expires: Instant,
}

pub struct OidcClient {
    client: CoreClient,
    pub provider_name: String,
    allowed_email: Option<String>,
    pending: Mutex<HashMap<String, PendingState>>,
    one_time_codes: Mutex<HashMap<String, (String, Instant)>>,
}

pub async fn init(cfg: &crate::config::OidcConfig) -> Result<OidcClient, String> {
    let issuer = IssuerUrl::new(cfg.issuer_url.clone())
        .map_err(|e| format!("invalid issuer_url: {e}"))?;

    let metadata = CoreProviderMetadata::discover_async(issuer, async_http_client)
        .await
        .map_err(|e| format!("OIDC discovery failed for {}: {e}", cfg.issuer_url))?;

    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(cfg.client_id.clone()),
        Some(ClientSecret::new(cfg.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(cfg.redirect_uri.clone())
            .map_err(|e| format!("invalid redirect_uri: {e}"))?,
    );

    Ok(OidcClient {
        client,
        provider_name: cfg.provider_name.clone().unwrap_or_else(|| "SSO".to_string()),
        allowed_email: cfg.allowed_email.clone(),
        pending: Mutex::new(HashMap::new()),
        one_time_codes: Mutex::new(HashMap::new()),
    })
}

// GET /auth/oidc/login
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Redirect, StatusCode> {
    let oidc = state.oidc_client.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token, nonce) = oidc
        .client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    {
        let mut pending = oidc.pending.lock().unwrap();
        let now = Instant::now();
        pending.retain(|_, v| v.expires > now);
        pending.insert(
            csrf_token.secret().clone(),
            PendingState { pkce_verifier, nonce, expires: now + PENDING_TTL },
        );
    }

    Ok(Redirect::to(auth_url.as_str()))
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

// GET /auth/oidc/callback
pub async fn callback_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CallbackParams>,
) -> Result<Redirect, StatusCode> {
    let oidc = state.oidc_client.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    if params.error.is_some() {
        tracing::warn!("OIDC provider returned error: {:?}", params.error);
        return Ok(Redirect::to("/?oidc_error=1"));
    }

    let code = params.code.ok_or(StatusCode::BAD_REQUEST)?;
    let csrf_state = params.state.ok_or(StatusCode::BAD_REQUEST)?;

    let pending_state = {
        let mut pending = oidc.pending.lock().unwrap();
        pending.remove(&csrf_state).ok_or(StatusCode::BAD_REQUEST)?
    };

    if Instant::now() > pending_state.expires {
        return Ok(Redirect::to("/?oidc_error=1"));
    }

    let token_response = oidc
        .client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pending_state.pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            tracing::warn!("OIDC token exchange failed: {e}");
            StatusCode::UNAUTHORIZED
        })?;

    let id_token = token_response.id_token().ok_or_else(|| {
        tracing::warn!("OIDC: no ID token in response");
        StatusCode::UNAUTHORIZED
    })?;

    let claims = id_token
        .claims(&oidc.client.id_token_verifier(), &pending_state.nonce)
        .map_err(|e| {
            tracing::warn!("OIDC ID token validation failed: {e}");
            StatusCode::UNAUTHORIZED
        })?;

    if let Some(allowed) = &oidc.allowed_email {
        let email = claims.email().map(|e| e.as_str()).unwrap_or("");
        if email != allowed.as_str() {
            tracing::warn!("OIDC: rejected email '{email}'");
            return Ok(Redirect::to("/?oidc_error=forbidden"));
        }
    }

    let session_token = state.sessions.create();
    let one_time_code = crate::key::random_hex_key();

    {
        let mut codes = oidc.one_time_codes.lock().unwrap();
        let now = Instant::now();
        codes.retain(|_, (_, exp)| *exp > now);
        codes.insert(one_time_code.clone(), (session_token, now + CODE_TTL));
    }

    Ok(Redirect::to(&format!("/?oidc_code={one_time_code}")))
}

#[derive(Deserialize)]
pub struct ExchangeBody {
    code: String,
}

#[derive(Serialize)]
pub struct ExchangeResponse {
    token: String,
}

// POST /auth/oidc/exchange
pub async fn exchange_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ExchangeBody>,
) -> Result<Json<ExchangeResponse>, StatusCode> {
    let oidc = state.oidc_client.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let session_token = {
        let mut codes = oidc.one_time_codes.lock().unwrap();
        let now = Instant::now();
        codes.retain(|_, (_, exp)| *exp > now);
        let (token, _) = codes.remove(&body.code).ok_or(StatusCode::UNAUTHORIZED)?;
        token
    };

    Ok(Json(ExchangeResponse { token: session_token }))
}

// GET /api/auth/config  (public)
#[derive(Serialize)]
pub struct AuthConfig {
    pub oidc_enabled: bool,
    pub provider_name: Option<String>,
}

pub async fn auth_config_handler(
    State(state): State<Arc<AppState>>,
) -> Json<AuthConfig> {
    Json(AuthConfig {
        oidc_enabled: state.oidc_client.is_some(),
        provider_name: state.oidc_client.as_ref().map(|c| c.provider_name.clone()),
    })
}
