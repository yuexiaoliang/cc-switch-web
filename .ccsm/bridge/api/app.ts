//! Mock for `@tauri-apps/api/app`.
//!
//! `getVersion()` returns the version the server reports via
//! `/api/version`. We cache the result on first call - the binary
//! version does not change for the lifetime of the page.

let cached: string | null = null;

/** Return the server''s `CARGO_PKG_VERSION`. Cached. */
export async function getVersion(): Promise<string> {
  if (cached !== null) return cached;
  try {
    const res = await fetch("/api/version");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const json = (await res.json()) as { version?: string };
    cached = json.version ?? "0.0.0";
  } catch {
    cached = "0.0.0";
  }
  return cached;
}

/** Return the build name. cc-switch-mini always reports
 *  `"cc-switch-mini"`. */
export async function getName(): Promise<string> {
  return "cc-switch-mini";
}

/** Return the Tauri identifier. We use the package name + version. */
export async function getTauriVersion(): Promise<string> {
  return "cc-switch-mini-1.0.0";
}
