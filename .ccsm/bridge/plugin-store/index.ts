//! Mock for `@tauri-apps/plugin-store`.
//!
//! The upstream Tauri store is a disk-backed key-value store. In
//! cc-switch-mini we drop down to `localStorage` which has the same
//! surface (get / set / save / has / delete) and is supported in
//! every browser. The values are JSON-encoded so the upstream code
//! can keep using `store.get(key) as SomeType` patterns.

export interface StoreOptions {
  /** Override the storage key prefix. Defaults to `cc-switch-mini`. */
  key?: string;
}

class LocalStorageStore {
  readonly id: string;
  private prefix: string;
  private cache: Map<string, unknown> = new Map();

  constructor(idOrPath: string, _opts?: StoreOptions) {
    // The upstream plugin accepts a file path; we only care about the
    // base name. This keeps the upstream call sites intact
    // (`Store.load("app_paths.json")` etc.).
    const last = idOrPath.split(/[\\/]/).pop() ?? idOrPath;
    this.id = last.replace(/\.[^.]+$/, "");
    this.prefix = `cc-switch-mini:${this.id}:`;
  }

  get<T>(key: string): T | undefined {
    if (this.cache.has(key)) return this.cache.get(key) as T;
    const raw = readLocal(this.prefix + key);
    if (raw === null) return undefined;
    try {
      const parsed = JSON.parse(raw) as T;
      this.cache.set(key, parsed);
      return parsed;
    } catch {
      return undefined;
    }
  }

  set(key: string, value: unknown): void {
    this.cache.set(key, value);
    writeLocal(this.prefix + key, JSON.stringify(value));
  }

  has(key: string): boolean {
    return this.get(key) !== undefined;
  }

  delete(key: string): void {
    this.cache.delete(key);
    removeLocal(this.prefix + key);
  }

  async clear(): Promise<void> {
    this.cache.clear();
    const all = listLocal();
    for (const k of all) {
      if (k.startsWith(this.prefix)) removeLocal(k);
    }
  }

  async save(): Promise<void> {
    // localStorage writes are synchronous; this stub satisfies the
    // upstream API shape.
  }

  /** Reload from disk. No-op. */
  async load(): Promise<void> {
    this.cache.clear();
  }

  /** No-op for compatibility. */
  async close(): Promise<void> {
    /* nothing to do */
  }
}

function readLocal(key: string): string | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}
function writeLocal(key: string, value: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(key, value);
  } catch {
    /* quota / privacy mode - ignore */
  }
}
function removeLocal(key: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(key);
  } catch {
    /* ignore */
  }
}
function listLocal(): string[] {
  if (typeof window === "undefined") return [];
  const out: string[] = [];
  for (let i = 0; i < window.localStorage.length; i++) {
    const k = window.localStorage.key(i);
    if (k) out.push(k);
  }
  return out;
}

export const Store = {
  /** Async load (mirrors the upstream `Store.load`). */
  async load(idOrPath: string, opts?: StoreOptions): Promise<LocalStorageStore> {
    return new LocalStorageStore(idOrPath, opts);
  },
};

/** Convenience re-export of the class for callers that `new Store(...)`
 *  directly. */
export { LocalStorageStore };
