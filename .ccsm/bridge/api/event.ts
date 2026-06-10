//! Mock for `@tauri-apps/api/event`.
//!
//! The bridge speaks to the upstream `EventBus` (a Tokio broadcast
//! channel exposed as Server-Sent Events on `/api/events`). Each
//! subscriber opens its own `EventSource` so the upstream back-pressure
//! semantics are preserved.

import { getEventsUrl } from "./shared";

export type UnlistenFn = () => void;

export interface Event<T> {
  event: string;
  id: number;
  payload: T;
}

interface Listener {
  event: string;
  handler: (event: Event<unknown>) => void;
}

const listeners: Set<Listener> = new Set();

let es: EventSource | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let backoffMs = 1000;

/** Open the SSE connection. Idempotent. */
function ensureConnected(): void {
  if (es) return;
  if (typeof window === "undefined") return;

  // EventSource does not support custom headers, so the token is
  // propagated via a query string when needed. The server is permissive
  // about the query string fallback; see `events::sse` for the matching
  // server-side check.
  const url = new URL(getEventsUrl());
  const token = (window as unknown as { __CCS_MINI_TOKEN__?: string })
    .__CCS_MINI_TOKEN__;
  const meta = document.querySelector('meta[name="ccs-token"]');
  if (token || meta) {
    url.searchParams.set("token", (token || meta?.getAttribute("content")) ?? "");
  }
  es = new EventSource(url.toString());

  es.onmessage = (raw: MessageEvent<string>) => {
    let parsed: { event?: string; payload?: unknown } | null = null;
    try {
      parsed = JSON.parse(raw.data);
    } catch {
      return;
    }
    if (!parsed || typeof parsed.event !== "string") return;
    for (const listener of listeners) {
      if (listener.event !== parsed.event) continue;
      try {
        listener.handler({
          event: parsed.event,
          id: 0,
          payload: parsed.payload,
        });
      } catch (err) {
        console.error("[cc-switch-mini] event handler threw", err);
      }
    }
  };

  es.onerror = () => {
    // EventSource will auto-reconnect; we just reset our state.
    if (es) {
      es.close();
      es = null;
    }
    if (reconnectTimer) clearTimeout(reconnectTimer);
    reconnectTimer = setTimeout(() => {
      backoffMs = Math.min(backoffMs * 2, 30000);
      ensureConnected();
    }, backoffMs);
  };

  es.onopen = () => {
    backoffMs = 1000;
  };
}

/**
 * Subscribe to a Tauri-style event. Returns an `UnlistenFn` that
 * removes the subscription. The bridge groups listeners by event name
 * so a single `EventSource` feeds all subscriptions.
 */
export async function listen<T>(
  event: string,
  handler: (event: Event<T>) => void,
  _options?: unknown,
): Promise<UnlistenFn> {
  const listener: Listener = {
    event,
    handler: handler as (event: Event<unknown>) => void,
  };
  listeners.add(listener);
  ensureConnected();
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && es) {
      es.close();
      es = null;
    }
  };
}

/**
 * Emit an event from the web side. cc-switch-mini does not have a
 * matching upstream command, so this is a no-op - the upstream Tauri
 * code never called `emit` from the renderer.
 */
export async function emit(_event: string, _payload?: unknown): Promise<void> {
  return;
}

/** Once variant - subscribes once and then unsubscribes. */
export async function once<T>(
  event: string,
  handler: (event: Event<T>) => void,
  options?: unknown,
): Promise<UnlistenFn> {
  const off = await listen<T>(
    event,
    (e) => {
      off();
      handler(e);
    },
    options,
  );
  return off;
}
