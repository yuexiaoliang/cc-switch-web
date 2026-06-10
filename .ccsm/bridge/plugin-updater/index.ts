//! Mock for `@tauri-apps/plugin-updater`.
//!
//! cc-switch-mini does not auto-update (the install script does that).
//! Returning `null` from `check` is the documented "no update
//! available" signal so the upstream UI shows the existing "up to
//! date" state.

export interface UpdateMetadata {
  version: string;
  notes?: string;
  pubDate?: string;
}

export interface Update {
  version: string;
  notes?: string;
  date?: string;
  downloadAndInstall: (onProgress?: (e: unknown) => void) => Promise<void>;
  download?: () => Promise<void>;
  install?: () => Promise<void>;
}

export interface CheckOptions {
  timeout?: number;
  channel?: string;
}

/** Always returns `null` - the spec defers version management to the
 *  install script. */
export async function check(_opts?: CheckOptions): Promise<Update | null> {
  return null;
}
