import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { homeDir, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl as openInBrowser } from "@tauri-apps/plugin-opener";
import type { Backend } from "./backend";
import { toCommandError } from "./errors";
import type {
  DiscoveredHost,
  EngineEvent,
  Profile,
  ProfileId,
  SecretKind,
  Settings,
  Snapshot,
  TrayLabels,
} from "./types";

/** Talks to the Rust core through Tauri's IPC. */
export class TauriBackend implements Backend {
  private async call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    try {
      return await invoke<T>(command, args);
    } catch (raw) {
      throw toCommandError(raw);
    }
  }

  snapshot = () => this.call<Snapshot>("snapshot");

  onEvent = (handler: (event: EngineEvent) => void) =>
    listen<EngineEvent>("engine", (event) => handler(event.payload));

  saveProfile = (profile: Profile) => this.call<Profile>("save_profile", { profile });
  deleteProfile = (id: ProfileId) => this.call<void>("delete_profile", { id });
  discoverSshHosts = () => this.call<DiscoveredHost[]>("discover_ssh_hosts");
  connect = (id: ProfileId) => this.call<void>("connect", { id });
  disconnect = (id: ProfileId) => this.call<void>("disconnect", { id });

  answerHostKey = (requestId: string, trust: boolean) =>
    this.call<boolean>("answer_host_key", { requestId, trust });
  answerSecret = (requestId: string, value: string | null, remember: boolean) =>
    this.call<boolean>("answer_secret", { requestId, value, remember });

  savedSecrets = (id: ProfileId) => this.call<SecretKind[]>("saved_secrets", { id });
  forgetSecrets = (id: ProfileId) => this.call<void>("forget_secrets", { id });
  forgetHostKey = (id: ProfileId) => this.call<boolean>("forget_host_key", { id });

  setSettings = (settings: Settings) => this.call<void>("set_settings", { settings });
  autostartEnabled = () => this.call<boolean>("autostart_enabled");
  setAutostart = (enabled: boolean) => this.call<void>("set_autostart", { enabled });
  setTrayLabels = (labels: TrayLabels) => this.call<void>("set_tray_labels", { labels });

  async pickKeyFile(): Promise<string | null> {
    const home = await homeDir().catch(() => null);
    const defaultPath = home ? await join(home, ".ssh").catch(() => undefined) : undefined;
    const picked = await open({ multiple: false, directory: false, defaultPath });
    if (typeof picked !== "string") return null;
    // `~` keeps the profile valid if the home directory moves, and reads better.
    return home && picked.startsWith(home) ? `~${picked.slice(home.length)}` : picked;
  }

  async openUrl(url: string): Promise<void> {
    try {
      await openInBrowser(url);
    } catch (raw) {
      throw toCommandError(raw);
    }
  }

  uiReady = () => this.call<void>("ui_ready");
  quit = () => this.call<void>("quit");
}
