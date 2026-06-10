//! Mock for `@tauri-apps/api/window`.
//!
//! cc-switch-mini runs inside the browser, so there is no real window
//! to control. The exported `getCurrentWindow` returns a no-op stub
//! that satisfies the upstream call sites without actually touching
//! the DOM beyond a couple of harmless effects (e.g. closing the tab
//! if the user explicitly asks for it via `exit()`).

export interface WindowLike {
  /** Show or hide window decorations. No-op in a browser. */
  setDecorations(_on: boolean): Promise<void>;
  /** Minimise the window. The browser cannot do this, so we blur
   *  the active element instead - it is the closest visual hint. */
  minimize(): Promise<void>;
  /** Maximise / un-maximise. No-op. */
  maximize(): Promise<void>;
  /** Un-maximise. No-op. */
  unmaximize(): Promise<void>;
  /** Query the current maximise state. Always `false` in a browser. */
  isMaximized(): Promise<boolean>;
  /** Toggle maximise. No-op. */
  toggleMaximize(): Promise<void>;
  /** Close the window. We delegate to `window.close()` which the
   * browser will only honour for windows opened by script. In the
   * common case we just print a console message. */
  close(): Promise<void>;
  /** Show. No-op. */
  show(): Promise<void>;
  /** Hide. No-op. */
  hide(): Promise<void>;
  /** Listen for window events. The upstream code only attaches a few
   * listeners (resize, focus) - we return a working unlisten fn. */
  onResized(handler: () => void): Promise<() => void>;
  onFocusChanged(handler: () => void): Promise<() => void>;
}

class BrowserWindow implements WindowLike {
  async setDecorations(_on: boolean): Promise<void> {
    // No-op - the browser controls chrome.
  }
  async minimize(): Promise<void> {
    if (typeof window !== "undefined" && "blur" in window) {
      window.blur();
    }
  }
  async maximize(): Promise<void> {
    // No-op. The browser handles full-screen via F11 / the API.
  }
  async unmaximize(): Promise<void> {
    // No-op.
  }
  async isMaximized(): Promise<boolean> {
    // The upstream Tauri code toggles a flag based on the OS window
    // state. In a browser the closest concept is document.fullscreen,
    // but the upstream UI treats `false` as "not maximised" and that
    // matches the common case for cc-switch-mini.
    return false;
  }
  async toggleMaximize(): Promise<void> {
    // No-op. If we wanted to be fancy we could toggle the browser
    // full-screen API, but the upstream code uses the boolean to
    // decide whether to show a "maximised" indicator in the title
    // bar - the browser already shows the right chrome.
  }
  async close(): Promise<void> {
    if (typeof window === "undefined") return;
    // Browsers only honour window.close() for windows opened via
    // window.open(). For a normal tab we just log - the user can
    // close it themselves.
    try {
      window.close();
    } catch {
      /* ignored */
    }
  }
  async show(): Promise<void> {
    if (typeof window !== "undefined" && "focus" in window) {
      window.focus();
    }
  }
  async hide(): Promise<void> {
    // No-op. Tab visibility is controlled by the browser chrome.
  }
  async onResized(handler: () => void): Promise<() => void> {
    if (typeof window === "undefined") return () => {};
    window.addEventListener("resize", handler);
    return () => window.removeEventListener("resize", handler);
  }
  async onFocusChanged(handler: () => void): Promise<() => void> {
    if (typeof window === "undefined") return () => {};
    window.addEventListener("focus", handler);
    window.addEventListener("blur", handler);
    return () => {
      window.removeEventListener("focus", handler);
      window.removeEventListener("blur", handler);
    };
  }
}

/** Return a handle to the (only) browser window. */
export function getCurrentWindow(): WindowLike {
  return new BrowserWindow();
}

/** Convenience alias used by some upstream modules. */
export const getCurrent = getCurrentWindow;
