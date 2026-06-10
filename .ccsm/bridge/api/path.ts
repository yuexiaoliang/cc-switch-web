//! Mock for `@tauri-apps/api/path`.
//!
//! The upstream `path` module is a *pure* helper - it does not call
//! the host process. Most of its functions compute absolute paths
//! given a base directory; we mirror the behaviour with simple
//! string ops since cc-switch-mini runs in the browser and cannot
//! query the host filesystem. The frontend uses these helpers to
//! build display paths, so a sensible fallback is good enough.

/** Return the user''s home directory. cc-switch-mini exposes a
 *  meta tag with the host''s home (set by the install script);
 *  otherwise we fall back to an empty string. */
export async function homeDir(): Promise<string> {
  if (typeof document !== "undefined") {
    const meta = document.querySelector('meta[name="ccs-home"]');
    if (meta) {
      const value = meta.getAttribute("content");
      if (value) return value;
    }
  }
  return "";
}

/** Join path segments. The browser only sees forward slashes, so we
 *  normalise. */
export async function join(...parts: string[]): Promise<string> {
  return parts
    .filter((p) => p && p.length > 0)
    .join("/")
    .replace(/\/{2,}/g, "/")
    .replace(/\/$/, "");
}

/** Return the directory containing the given path. */
export async function dirname(path: string): Promise<string> {
  if (!path) return "";
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx <= 0 ? "" : path.slice(0, idx);
}

/** Return the last segment of the path. */
export async function basename(path: string, ext?: string): Promise<string> {
  if (!path) return "";
  const last = path.split(/[\\/]/).pop() ?? "";
  if (ext && last.endsWith(ext)) return last.slice(0, -ext.length);
  return last;
}

/** Return the file extension (including the leading dot). */
export async function extname(path: string): Promise<string> {
  const base = await basename(path);
  const idx = base.lastIndexOf(".");
  return idx <= 0 ? "" : base.slice(idx);
}

/** Return the per-app config directory. The server publishes these
 *  via meta tags; the bridge just reads them. */
export async function appConfigDir(): Promise<string> {
  if (typeof document !== "undefined") {
    const meta = document.querySelector('meta[name="ccs-app-config-dir"]');
    if (meta) {
      const value = meta.getAttribute("content");
      if (value) return value;
    }
  }
  return (await homeDir()) + "/.cc-switch";
}

/** Return the per-app local data directory. */
export async function appLocalDataDir(): Promise<string> {
  return (await homeDir()) + "/.local/share/cc-switch";
}

/** Return the per-app data directory. */
export async function appDataDir(): Promise<string> {
  return (await homeDir()) + "/.local/share/cc-switch";
}

/** Return the per-app cache directory. */
export async function appCacheDir(): Promise<string> {
  return (await homeDir()) + "/.cache/cc-switch";
}

/** Return the per-app log directory. */
export async function appLogDir(): Promise<string> {
  return (await homeDir()) + "/.local/share/cc-switch/logs";
}
