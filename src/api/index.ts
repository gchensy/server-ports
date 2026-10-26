import { isTauri } from "@tauri-apps/api/core";
import type { Backend } from "./backend";
import { TauriBackend } from "./tauriBackend";

/**
 * The desktop shell talks to the Rust core. In a plain browser during development there is no
 * core, so a pretend one stands in (it is not part of production builds).
 */
export async function createBackend(): Promise<Backend> {
  if (isTauri()) return new TauriBackend();
  if (import.meta.env.DEV) {
    const { MockBackend } = await import("./mockBackend");
    return new MockBackend();
  }
  throw new Error("Server Ports has to run inside its desktop shell");
}

export type { Backend };
