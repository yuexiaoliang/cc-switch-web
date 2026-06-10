//! Mock for `@tauri-apps/plugin-process`.

/** Exit the application. We cannot actually exit the browser, so
 *  we just log and rely on the user closing the tab. */
export async function exit(_code: number = 0): Promise<void> {
  // The upstream `restart_app` command is dispatched to the server,
  // which would normally kill + relaunch the Tauri process. In
  // cc-switch-mini there is nothing to relaunch; we just close the
  // current tab if we can.
  if (typeof window !== "undefined") {
    try {
      window.close();
    } catch {
      /* ignored */
    }
  }
}

/** Relaunch the application. No-op. */
export async function relaunch(): Promise<void> {
  // Same as exit() - the user is expected to refresh the page.
  await exit(0);
}
