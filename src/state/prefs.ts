/** Small per-user choices that live in the webview's storage, not in the Rust settings. */
export function readPref(key: string): string | null {
  try {
    return window.localStorage.getItem(`server-ports.${key}`);
  } catch {
    return null;
  }
}

export function writePref(key: string, value: string | null): void {
  try {
    if (value === null) window.localStorage.removeItem(`server-ports.${key}`);
    else window.localStorage.setItem(`server-ports.${key}`, value);
  } catch {
    // Storage can be unavailable (private mode, blocked); the app works without it.
  }
}
