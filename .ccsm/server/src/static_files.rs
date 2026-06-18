//! Serve the embedded Vite build output.
//!
//! We embed `dist/` (the upstream Vite build output) into the binary at
//! compile time via the `include_dir!` macro. At runtime we look up paths
//! under `DIST` and fall back to `index.html` for SPA routes.
//!
//! To keep the binary self-contained when `dist/` is missing (e.g. during
//! `cargo check` before `npm run build`), we fall back to an in-memory
//! placeholder page that explains how to populate `dist/`.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use include_dir::{include_dir, Dir};
use std::sync::Arc;

/// Embed the Vite output. The path is relative to this source file at
/// compile time, so the build script just needs to drop the assets next
/// to it.
pub static DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../dist");

/// HTTP handler. Maps the request path to a file inside `DIST`. When the
/// path does not match a real file (e.g. `/providers` deep link) we fall
/// back to `index.html` so the React router can pick it up.
pub async fn serve(State(ctx): State<Arc<crate::AppContext>>, uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // 1. Try to serve the exact file (e.g. /assets/index-abc.js).
    if let Some(file) = DIST.get_file(path) {
        return file_response(file);
    }

    // 2. SPA fallback: hand the React router the index page.
    if ctx.opts.spa_fallback {
        if let Some(index) = DIST.get_file("index.html") {
            return file_response(index);
        }
    }

    // 3. dist/ was never built (development workflow). Reply with a
    //    friendly placeholder so the server is still useful for poking at
    //    the API surface.
    placeholder_response()
}

fn file_response(file: &include_dir::File<'_>) -> Response {
    let mime = mime_guess::from_path(file.path()).first_or_octet_stream();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        // The Vite output is content-hashed, so the browser can cache it
        // forever. `index.html` and friends get short caching instead.
        .header(
            header::CACHE_CONTROL,
            if is_long_lived_asset(file.path()) {
                "public, max-age=31536000, immutable"
            } else {
                "public, max-age=0, must-revalidate"
            },
        )
        .body(Body::from(file.contents().to_vec()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn is_long_lived_asset(path: &std::path::Path) -> bool {
    // Vite fingerprints anything under `assets/` with a content hash. The
    // few un-hashed files (favicon, robots.txt) keep short caching.
    path.starts_with("assets") || path.extension().is_some()
}

fn placeholder_response() -> Response {
    let body = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>cc-switch-web</title>
    <style>
      body { font-family: -apple-system, system-ui, sans-serif; max-width: 720px; margin: 4rem auto; padding: 0 1.5rem; line-height: 1.5; color: #1f2937; }
      code { background: #f3f4f6; padding: 0.1em 0.3em; border-radius: 3px; }
      h1 { margin-bottom: 0.25rem; }
      .muted { color: #6b7280; }
    </style>
  </head>
  <body>
    <h1>cc-switch-web is running</h1>
    <p class="muted">The static frontend bundle has not been built yet.</p>
    <p>From the repository root run:</p>
    <pre><code>npm install
npm run build:renderer
cargo build --release -p cc-switch-web-server</code></pre>
    <p>Then restart the server and reload this page.</p>
  </body>
</html>"#;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

/// Convenience: serve a 200 with a body. Used by other handlers.
#[allow(dead_code)]
pub fn ok_html(body: &'static str) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

/// Convenience: serve a 404 with a body.
#[allow(dead_code)]
pub fn not_found(_req: Request) -> Response {
    StatusCode::NOT_FOUND.into_response()
}
