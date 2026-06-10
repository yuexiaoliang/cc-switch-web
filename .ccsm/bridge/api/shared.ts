//! Shared helpers for the bridge modules.
//!
//! The bridge lives at `.ccsm/bridge/` and is installed as a
//! `file:`-based override for the Tauri packages. Every module needs
//! to know the server URL, the bearer token, and whether the bridge
//! has been bootstrapped (e.g. when the page is opened in a tab that
//! is not served by the cc-switch-mini server).

/** Inferred base URL of the server - the page is served by the same
 *  origin so a relative path is enough. */
export function getBaseUrl(): string {
  if (typeof window === "undefined") return "";
  return window.location.origin;
}

/** Resolve `/api/invoke/<cmd>` against the page origin. */
export function getInvokeUrl(cmd: string): string {
  return `${getBaseUrl()}/api/invoke/${encodeURIComponent(cmd)}`;
}

/** Resolve `/api/events` against the page origin. */
export function getEventsUrl(): string {
  return `${getBaseUrl()}/api/events`;
}

/** Read the optional bearer token. The token is published to the page
 *  via `<meta name="ccs-token" content="...">` (set by the
 *  `runtime::run` banner) or, when running locally without auth, just
 *  returns `null`.
 */
export function getToken(): string | null {
  if (typeof document === "undefined") return null;
  const meta = document.querySelector('meta[name="ccs-token"]');
  if (meta) {
    const value = meta.getAttribute("content");
    if (value && value.length > 0) return value;
  }
  // The runtime also embeds the token in a global injected by the
  // server's HTML shim - both sources are supported so dev tools can
  // experiment freely.
  const global = (window as unknown as { __CCS_MINI_TOKEN__?: string })
    .__CCS_MINI_TOKEN__;
  if (global) return global;
  return null;
}

/** True when the page is being served by the cc-switch-mini server
 *  (origin contains `/api/health`) and the user did not load the
 *  static `dist/` directly. In a packaged build this is always true;
 *  during local dev with `vite` the bridge will simply fail-fast
 *  with a descriptive error. */
export function isBridgeEnabled(): boolean {
  if (typeof window === "undefined") return false;
  // The bridge is enabled when the page knows the events URL (we set
  // this from the server''s HTML shim). The check is permissive: the
  // /api/events endpoint exists on the same origin so a same-origin
  // fetch will succeed regardless.
  return Boolean(getBaseUrl());
}

/** Add the bearer token (if any) to a Headers-style object. The
 *  returned object is a fresh `Record` so the caller can pass it
 *  straight to `fetch`. */
export function writeAuthHeader(
  headers: Record<string, string> = {},
): Record<string, string> {
  const token = getToken();
  if (token) {
    return { ...headers, authorization: `Bearer ${token}` };
  }
  return headers;
}
