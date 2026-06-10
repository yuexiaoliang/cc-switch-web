//! Event broadcasting.
//!
//! Mirrors the Tauri `app_handle.emit(...)` mechanism. Handlers that mutate
//! state push a `FrontendEvent` onto the broadcast bus; the SSE handler
//! forwards each event to all subscribed clients as a JSON-encoded `data:`
//! line.
//!
//! We deliberately keep the event names and payload shapes identical to the
//! upstream Tauri commands so the bridge layer does not need translation
//! logic.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Response,
};
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Channel capacity: 64 events buffered per subscriber. Senders drop oldest
/// when a slow consumer is behind, which is fine - the frontend is best
/// effort and re-fetches state on reconnect.
const CHANNEL_CAPACITY: usize = 64;

/// One event the frontend listens for. Field names mirror the upstream Tauri
/// `emit` calls (camelCase).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "payload")]
pub enum FrontendEvent {
    /// Emitted by `ProviderService::switch` after a successful provider
    /// switch. Frontend uses this to invalidate the `getCurrent` cache.
    #[serde(rename = "provider-switched")]
    ProviderSwitched {
        #[serde(rename = "appType")]
        app_type: String,
        #[serde(rename = "providerId")]
        provider_id: String,
    },
    /// Emitted by the universal provider sync path.
    #[serde(rename = "universal-provider-synced")]
    UniversalProviderSynced { action: String, id: String },
    /// Emitted when the usage cache is updated by a script query.
    #[serde(rename = "usage-cache-updated")]
    UsageCacheUpdated {
        kind: String,
        #[serde(rename = "appType")]
        app_type: String,
        #[serde(rename = "providerId")]
        provider_id: String,
        data: serde_json::Value,
    },
}

/// Broadcast bus handle. Cheap to clone; clones share the same channel.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<FrontendEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self { tx }
    }

    /// Publish an event to every subscriber. Silently drops the message when
    /// there are no active subscribers - the frontend will catch up on the
    /// next state fetch.
    pub fn publish(&self, event: FrontendEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to the broadcast bus. Returns a `broadcast::Receiver` which
    /// implements `Stream` via the `StreamExt` trait.
    pub fn subscribe(&self) -> broadcast::Receiver<FrontendEvent> {
        self.tx.subscribe()
    }

    /// Number of live subscribers. Used by tests and by the `/api/health`
    /// endpoint to surface the bus health.
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE handler. Each connected browser gets its own subscription so a slow
/// client cannot block the bus for everyone else.
pub async fn sse(State(ctx): State<Arc<crate::AppContext>>, headers: HeaderMap) -> Response {
    // Reuse the same bearer-token check as the dispatch middleware.
    if !crate::auth::authorise(ctx.opts.token.as_deref(), &headers) {
        return crate::auth::unauthorized();
    }

    let rx = ctx.events.subscribe();

    let stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            tokio::select! {
                biased;
                msg = rx.recv() => {
                    match msg {
                        Ok(event) => {
                            let json = match serde_json::to_string(&event) {
                                Ok(s) => s,
                                Err(e) => {
                                    log::error!("failed to serialise event: {e}");
                                    continue;
                                }
                            };
                            yield Ok::<_, Infallible>(Event::default().data(json));
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("SSE client lagged, skipped {n} events");
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                // 25-second heartbeat matches what nginx/caddy expect.
                _ = tokio::time::sleep(std::time::Duration::from_secs(25)) => {
                    yield Ok::<_, Infallible>(Event::default().comment("keep-alive"));
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::new())
        .into_response()
}
