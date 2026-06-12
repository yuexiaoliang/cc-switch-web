//! cc-switch-mini headless server.
//!
//! This crate is a thin Axum-based HTTP adapter that exposes the upstream
//! `cc_switch_lib` business logic to a browser. It does not own any of the
//! provider / proxy / database semantics; it just routes JSON-RPC-style
//! requests (`POST /api/invoke/<cmd>`) to the upstream services and streams
//! the resulting Tauri-style events back via SSE (`GET /api/events`).
//!
//! Architecture (mirrors section 5 of `cc-switch-mini.md`):
//!
//! ```text
//! browser  --HTTP-->  Axum router
//!                      |
//!                      +-- POST /api/invoke/*  --> dispatch::invoke()
//!                      |     |
//!                      |     +-- ProviderService  (cc_switch_lib)
//!                      |     +-- ProxyService
//!                      |     +-- ConfigService / settings
//!                      |     +-- StreamCheckService
//!                      |     +-- Database
//!                      |
//!                      +-- GET  /api/events    --> events::sse()
//!                      +-- GET  /<path>        --> static_files::serve()
//! ```

pub mod auth;
pub mod commands_extra;
pub mod cli;
pub mod dispatch;
pub mod error;
pub mod events;
pub mod runtime;
pub mod state;
pub mod static_files;

use axum::{
    middleware,
    routing::{any, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// Top-level shared state handed to every Axum handler.
#[derive(Clone)]
pub struct AppContext {
    /// Upstream business state (db + proxy service + usage cache).
    pub state: Arc<cc_switch_lib::AppState>,
    /// Tokio broadcast bus for events that the frontend listens to.
    pub events: events::EventBus,
    /// Resolved CLI options - needed by some handlers (data dir, port, ...).
    pub opts: Arc<cli::Resolved>,
    /// Embedded static-file directory.
    pub dist: &'static include_dir::Dir<'static>,
}

impl AppContext {
    pub fn new(
        state: Arc<cc_switch_lib::AppState>,
        events: events::EventBus,
        opts: Arc<cli::Resolved>,
    ) -> Self {
        Self {
            state,
            events,
            opts,
            dist: &static_files::DIST,
        }
    }
}

/// Build the Axum router. Exposed for integration tests.
pub fn build_router(ctx: Arc<AppContext>) -> Router {
    // CORS for dev convenience (running Vite against a different port). The
    // upstream frontend will be served from the same origin in production.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // The /api/* tree carries the bearer-token guard. The static fallback
    // stays unauthenticated so the browser''s initial asset load is
    // friction-free; the bridge layer will attach the token from the
    // `window.__CCS_MINI_TOKEN__` global.
    let api_routes = Router::new()
        .route("/api/invoke/:cmd", post(dispatch::invoke))
        .route("/api/events", get(events::sse))
        .route("/api/health", get(dispatch::health))
        .route("/api/version", get(dispatch::version))
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth::enforce));

    Router::new()
        .merge(api_routes)
        .fallback(any(static_files::serve))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-content-type-options"),
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(ctx)
}

/// Bind the router to `addr` and serve it until `ctrl_c` (or SIGTERM).
pub async fn serve(ctx: Arc<AppContext>, addr: std::net::SocketAddr) -> error::Result<()> {
    let app = build_router(ctx);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("listening on http://{addr}");

    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());
    server.await?;
    Ok(())
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM and return.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => log::info!("received SIGINT, shutting down"),
        _ = terminate => log::info!("received SIGTERM, shutting down"),
    }
}

#[cfg(test)]
impl AppContext {
    /// A placeholder context for unit tests. Not safe to call DB or
    /// service methods on this — handlers that need the real state
    /// must use a fixture.
    pub fn placeholder() -> Self {
        use crate::events::EventBus;
        // Build a throwaway AppContext backed by a memory database. Tests
        // that need real persistence should use the upstream `Database::memory`
        // helper instead.
        let db = cc_switch_lib::Database::init().expect("test database init");
        Self {
            state: Arc::new(cc_switch_lib::AppState::new(Arc::new(db))),
            events: EventBus::new(),
            opts: Arc::new(crate::cli::Resolved::placeholder()),
            dist: &static_files::DIST,
        }
    }
}
