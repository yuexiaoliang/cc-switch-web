//! Mock for `@tauri-apps/api/core`.
//!
//! The bridge re-exports `invoke` and `convertFileSrc` with the same
//! signatures the upstream package exports. `invoke` becomes a POST to
//! `/api/invoke/<cmd>`. `convertFileSrc` returns a fake `asset://` URL
//! - cc-switch-mini does not serve local files, so anything that asks
//! for a local-file URL gets a transparent 1x1 PNG.

import { getInvokeUrl, isBridgeEnabled, writeAuthHeader } from "./shared";

// Re-exported types: in cc-switch the import site is `import { invoke }
// from "@tauri-apps/api/core"` plus `import type { InvokeArgs }` etc.
// We keep the same surface as the upstream package.

/**
 * Invoke a Tauri command. Returns a Promise that resolves with the
 * command's return value (serialised by the server) or rejects with an
 * `Error` whose message matches the server's `error.message` field.
 */
export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown> | unknown[],
  _options?: unknown,
): Promise<T> {
  if (!isBridgeEnabled()) {
    throw new Error(
      `[cc-switch-mini] invoke("${cmd}") called before the bridge booted; \
       check that the page is served from the cc-switch-mini server`,
    );
  }
  // Tauri's IPC accepts both an object and a positional array. cc-switch
  // always uses the object form, so we normalise the input here for
  // safety.
  const argsObject = normaliseArgs(args);
  const url = getInvokeUrl(cmd);

  let response: Response;
  try {
    response = await fetch(url, {
      method: "POST",
      headers: writeAuthHeader({
        "content-type": "application/json",
      }),
      body: JSON.stringify({ args: argsObject }),
    });
  } catch (err) {
    throw new Error(
      `[cc-switch-mini] network error calling ${cmd}: ${(err as Error).message}`,
    );
  }

  // The dispatch layer always returns JSON. The success body shape is
  // the command's return value (or null). The error body shape is
  // `{ error: { code, message } }`.
  const text = await response.text();
  let payload: unknown = null;
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = text;
    }
  }
  if (!response.ok) {
    const err = (payload as { error?: { message?: string } } | null)?.error;
    throw new Error(err?.message ?? `[cc-switch-mini] ${cmd} failed: HTTP ${response.status}`);
  }
  return payload as T;
}

/**
 * Mocked equivalent of Tauri's `convertFileSrc`. cc-switch-mini does
 * not serve local files - we hand back a 1x1 transparent PNG so the
 * browser does not throw when the upstream UI tries to render an
 * image. Callers that rely on actually reading a local file will need
 * to be redesigned; the upstream frontend does not have any such
 * case for the P0 commands.
 */
export function convertFileSrc(filePath: string, _protocol = "asset"): string {
  if (!filePath) return "data:image/png;base64," + EMPTY_PNG;
  // If the caller passed a fully-qualified URL, return it unchanged.
  if (/^(https?|data|blob):/.test(filePath)) return filePath;
  if (filePath.startsWith("/api/")) return filePath;
  return "data:image/png;base64," + EMPTY_PNG;
}

const EMPTY_PNG =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

function normaliseArgs(args: unknown): Record<string, unknown> {
  if (!args) return {};
  if (typeof args === "object" && !Array.isArray(args)) {
    return args as Record<string, unknown>;
  }
  if (Array.isArray(args)) {
    // cc-switch only uses object form so this branch is defensive.
    return { args };
  }
  return {};
}

// Re-export the type the upstream package exports so `import type`
// lines do not break.
export type InvokeArgs = Record<string, unknown>;
