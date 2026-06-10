//! Mock for `@tauri-apps/plugin-dialog`.
//!
//! The upstream Tauri plugin surfaces native OS dialogs. In a browser
//! we degrade to `alert` / `confirm` so the upstream UI keeps working.
//! Returns a plain Promise so call sites do not need to branch.

/** Native "message box" (informational dialog). */
export async function message(
  message: string,
  options?: { title?: string; kind?: "info" | "warning" | "error" },
): Promise<void> {
  if (typeof window === "undefined") return;
  const text = options?.title
    ? `[${options.title}]\n\n${message}`
    : message;
  window.alert(text);
}

/** Yes / No question. Returns `true` for "yes". */
export async function ask(
  message: string,
  options?: { title?: string; kind?: "info" | "warning" | "error" },
): Promise<boolean> {
  if (typeof window === "undefined") return false;
  return window.confirm(options?.title ? `[${options.title}]\n\n${message}` : message);
}

/** OK / Cancel question. */
export async function confirm(
  message: string,
  options?: { title?: string; kind?: "info" | "warning" | "error" },
): Promise<boolean> {
  return ask(message, options);
}

/** File-picker. Always returns `null` in cc-switch-mini - the
 *  server has no file picker; users upload / download via the Web UI. */
export async function open(_options?: unknown): Promise<string | null> {
  return null;
}

export async function save(_options?: unknown): Promise<string | null> {
  return null;
}
