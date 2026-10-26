import type { Backend } from "../api/backend";
import { CommandError, toCommandError, type AppError } from "../api/errors";
import type {
  EngineEvent,
  LogLine,
  Profile,
  ProfileId,
  ProfileRuntime,
  Prompt,
  SecretKind,
  Settings,
  Snapshot,
  StateEvent,
} from "../api/types";
import type { I18n } from "../i18n";
import { readPref, writePref } from "./prefs";

const LOG_CAPACITY = 300;
const REMEMBERED_PROMPTS = 100;

export interface Toast {
  id: number;
  text: string;
}

export interface AppState {
  loaded: boolean;
  /** Why the first snapshot could not be had. */
  loadError: string | null;
  profiles: Profile[];
  settings: Settings;
  runtimes: Readonly<Record<ProfileId, ProfileRuntime>>;
  logs: Readonly<Record<ProfileId, readonly LogLine[]>>;
  /** Questions from the engine, oldest first; the first one is on screen. */
  prompts: readonly Prompt[];
  selectedId: ProfileId | null;
  toasts: readonly Toast[];
}

const initialState: AppState = {
  loaded: false,
  loadError: null,
  profiles: [],
  settings: { closeToTray: true },
  runtimes: {},
  logs: {},
  prompts: [],
  selectedId: null,
  toasts: [],
};

export function isActive(runtime: ProfileRuntime | undefined): boolean {
  const state = runtime?.conn.state;
  return state === "connecting" || state === "connected" || state === "reconnecting";
}

/** Everything the screens show, kept in step with the Rust core. */
export class AppStore {
  private current: AppState = initialState;
  private readonly listeners = new Set<() => void>();
  /** Questions that were withdrawn: a snapshot that raced with the withdrawal must not revive them. */
  private readonly withdrawn: string[] = [];
  private nextToast = 1;
  private stopListening: (() => void) | null = null;

  constructor(
    private readonly backend: Backend,
    private readonly i18n: I18n,
  ) {}

  // -- the store contract of useSyncExternalStore ---------------------------------------------

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  readonly getState = (): AppState => this.current;

  private update(change: (state: AppState) => Partial<AppState>): void {
    this.current = { ...this.current, ...change(this.current) };
    this.listeners.forEach((listener) => listener());
  }

  // -- keeping in step with the core ----------------------------------------------------------

  /** Listens first and asks for the snapshot second, so that nothing falls in between. */
  async start(): Promise<void> {
    const early: EngineEvent[] = [];
    let loaded = false;
    try {
      this.stopListening = await this.backend.onEvent((event) => {
        if (loaded) this.apply(event);
        else early.push(event);
      });
      const snapshot = await this.backend.snapshot();
      this.load(snapshot);
      early.forEach((event) => this.apply(event));
      loaded = true;
      this.update(() => ({ loaded: true, loadError: null }));
    } catch (raw) {
      this.stop();
      const error = toCommandError(raw);
      this.update(() => ({ loadError: error.message }));
    }
  }

  stop(): void {
    this.stopListening?.();
    this.stopListening = null;
  }

  private load(snapshot: Snapshot): void {
    const saved = readPref("selected");
    const selectedId =
      snapshot.profiles.find((p) => p.id === saved)?.id ?? snapshot.profiles[0]?.id ?? null;
    this.update(() => ({ profiles: snapshot.profiles, settings: snapshot.settings, selectedId }));
    snapshot.runtimes.forEach((runtime) => this.mergeRuntime(runtime));
    snapshot.logs.forEach((line) => this.mergeLog(line));
    snapshot.questions.forEach((question) => this.apply(question));
  }

  private apply(event: EngineEvent): void {
    switch (event.type) {
      case "state":
        this.mergeRuntime(runtimeOf(event));
        break;
      case "log":
        this.mergeLog({
          seq: event.seq,
          profileId: event.profileId,
          ts: event.ts,
          level: event.level,
          message: event.message,
        });
        break;
      case "hostKey":
      case "secret":
        this.addPrompt(event);
        break;
      case "promptClosed":
        this.withdraw(event.requestId);
        break;
    }
  }

  private mergeRuntime(runtime: ProfileRuntime): void {
    this.update((state) => {
      const known = state.runtimes[runtime.profileId];
      if (known && known.rev >= runtime.rev) return {};
      return { runtimes: { ...state.runtimes, [runtime.profileId]: runtime } };
    });
  }

  private mergeLog(line: LogLine): void {
    this.update((state) => {
      const lines = state.logs[line.profileId] ?? [];
      if (lines.some((known) => known.seq === line.seq)) return {};
      const merged = [...lines, line].sort((a, b) => a.seq - b.seq).slice(-LOG_CAPACITY);
      return { logs: { ...state.logs, [line.profileId]: merged } };
    });
  }

  private addPrompt(prompt: Prompt): void {
    this.update((state) => {
      const seen = state.prompts.some((p) => p.requestId === prompt.requestId);
      if (seen || this.withdrawn.includes(prompt.requestId)) return {};
      return { prompts: [...state.prompts, prompt] };
    });
  }

  private withdraw(requestId: string): void {
    this.withdrawn.push(requestId);
    if (this.withdrawn.length > REMEMBERED_PROMPTS) this.withdrawn.shift();
    this.update((state) => ({ prompts: state.prompts.filter((p) => p.requestId !== requestId) }));
  }

  // -- what the user can do -------------------------------------------------------------------

  select(id: ProfileId | null): void {
    writePref("selected", id);
    this.update(() => ({ selectedId: id }));
  }

  /** Runs a command and tells the user if it failed. */
  private async attempt<T>(job: () => Promise<T>): Promise<T | undefined> {
    try {
      return await job();
    } catch (raw) {
      this.fail(toCommandError(raw));
      return undefined;
    }
  }

  async connect(id: ProfileId): Promise<void> {
    await this.attempt(() => this.backend.connect(id));
  }

  async disconnect(id: ProfileId): Promise<void> {
    await this.attempt(() => this.backend.disconnect(id));
  }

  /** For a server that is waiting out its back-off. */
  async retryNow(id: ProfileId): Promise<void> {
    await this.attempt(async () => {
      await this.backend.disconnect(id);
      await this.backend.connect(id);
    });
  }

  /** Saves the profile. A profile the core refuses is thrown for the form to show. */
  async saveProfile(profile: Profile): Promise<Profile> {
    let saved: Profile;
    try {
      saved = await this.backend.saveProfile(profile);
    } catch (raw) {
      const error = toCommandError(raw);
      if (error.error.code !== "invalid") this.fail(error);
      throw error;
    }
    this.update((state) => {
      const known = state.profiles.some((p) => p.id === saved.id);
      const profiles = known
        ? state.profiles.map((p) => (p.id === saved.id ? saved : p))
        : [...state.profiles, saved];
      return { profiles };
    });
    this.select(saved.id);
    return saved;
  }

  async deleteProfile(id: ProfileId): Promise<void> {
    const deleted = await this.attempt(async () => {
      await this.backend.deleteProfile(id);
      return true;
    });
    if (!deleted) return;
    const state = this.current;
    const index = state.profiles.findIndex((p) => p.id === id);
    const profiles = state.profiles.filter((p) => p.id !== id);
    const runtimes = without(state.runtimes, id);
    const logs = without(state.logs, id);
    const selectedId =
      state.selectedId === id ? (profiles[Math.min(index, profiles.length - 1)]?.id ?? null) : state.selectedId;
    this.update(() => ({ profiles, runtimes, logs, selectedId }));
    writePref("selected", selectedId);
  }

  answerHostKey(requestId: string, trust: boolean): void {
    this.withdraw(requestId);
    void this.attempt(() => this.backend.answerHostKey(requestId, trust));
  }

  answerSecret(requestId: string, value: string | null, remember: boolean): void {
    this.withdraw(requestId);
    void this.attempt(() => this.backend.answerSecret(requestId, value, remember));
  }

  async savedSecrets(id: ProfileId): Promise<SecretKind[]> {
    return (await this.attempt(() => this.backend.savedSecrets(id))) ?? [];
  }

  async forgetSecrets(id: ProfileId): Promise<void> {
    await this.attempt(() => this.backend.forgetSecrets(id));
  }

  /** For a server that changed its key: forgets the old one and tries again. */
  async trustNewHostKey(id: ProfileId): Promise<void> {
    await this.attempt(async () => {
      await this.backend.forgetHostKey(id);
      await this.backend.connect(id);
    });
  }

  async setSettings(settings: Settings): Promise<void> {
    const previous = this.current.settings;
    this.update(() => ({ settings }));
    const saved = await this.attempt(async () => {
      await this.backend.setSettings(settings);
      return true;
    });
    if (!saved) this.update(() => ({ settings: previous }));
  }

  /** The tray menu is made by the core, from words the UI hands over. */
  async sendTrayLabels(): Promise<void> {
    await this.attempt(() => this.backend.setTrayLabels(this.i18n.trayLabels()));
  }

  // -- telling the user -----------------------------------------------------------------------

  toast(text: string): void {
    const toast = { id: this.nextToast++, text };
    this.update((state) => ({ toasts: [...state.toasts, toast] }));
    window.setTimeout(() => this.dismissToast(toast.id), 6000);
  }

  dismissToast(id: number): void {
    this.update((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
  }

  /** For callers that talk to the backend themselves: tells the user what went wrong. */
  reportError(raw: unknown): void {
    this.fail(toCommandError(raw));
  }

  private fail(error: CommandError): void {
    this.toast(this.describe(error.error));
  }

  private describe(error: AppError): string {
    const title = this.i18n.errorTitle(error.code === "invalid" ? "other" : error.code);
    return "detail" in error && error.detail ? `${title}: ${error.detail}` : title;
  }
}

function without<T>(record: Readonly<Record<string, T>>, key: string): Record<string, T> {
  const copy = { ...record };
  delete copy[key];
  return copy;
}

function runtimeOf(event: StateEvent): ProfileRuntime {
  return {
    profileId: event.profileId,
    conn: event.conn,
    forwards: event.forwards,
    connectedAt: event.connectedAt,
    rev: event.rev,
  };
}
