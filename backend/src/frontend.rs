use axum::{body::Body, http::{StatusCode, header}, response::{IntoResponse, Response}};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/build"]
struct Asset;

pub async fn handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Asset::get(path) {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, mime_for_path(path))
            .header(header::CACHE_CONTROL, cache_control(path))
            .body(Body::from(content.data))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        None => match Asset::get("index.html") {
            Some(content) => Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::from(content.data))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
            None => StatusCode::NOT_FOUND.into_response(),
        },
    }
}

fn cache_control(path: &str) -> &'static str {
    if path.starts_with("_app/immutable/") {
        // Content-hashed filenames — safe to cache forever.
        "public, max-age=31536000, immutable"
    } else if path == "service-worker.js" || path.ends_with(".html") {
        // The browser must always re-validate the SW script so it detects
        // new versions immediately. Same for HTML entry points.
        "no-cache"
    } else {
        // Manifest, icons, and other static files: short cache.
        "public, max-age=3600"
    }
}

fn mime_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html"  => "text/html; charset=utf-8",
        "css"   => "text/css",
        "js"    => "application/javascript",
        "json"  => "application/json",
        "svg"   => "image/svg+xml",
        "png"   => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico"   => "image/x-icon",
        "woff2" => "font/woff2",
        "woff"  => "font/woff",
        _       => "application/octet-stream",
    }
}
