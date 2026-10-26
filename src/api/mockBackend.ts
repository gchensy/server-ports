import type { Backend } from "./backend";
import { CommandError } from "./errors";
import type {
  DiscoveredHost,
  EngineEvent,
  ErrorInfo,
  Forward,
  ForwardRuntime,
  LogLevel,
  Profile,
  ProfileId,
  ProfileRuntime,
  SecretKind,
  Settings,
  Snapshot,
  TrayLabels,
} from "./types";

/** The password every pretend server accepts. */
export const MOCK_PASSWORD = "hunter2";
const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

function forward(id: string, label: string, local: number, remote: number, remoteHost = "127.0.0.1"): Forward {
  return { id, label, bindHost: "127.0.0.1", localPort: local, remoteHost, remotePort: remote };
}

function seedProfiles(): Profile[] {
  const base = { port: 22, autoReconnect: true, connectOnLaunch: false };
  return [
    {
      ...base,
      id: "prod",
      name: "Production database",
      host: "db.example.com",
      username: "deploy",
      auth: { type: "key", path: "~/.ssh/id_ed25519" },
      forwards: [forward("prod-pg", "Postgres", 5432, 5432), forward("prod-redis", "Redis", 6379, 6379, "10.0.2.15")],
      connectOnLaunch: true,
    },
    {
      ...base,
      id: "staging",
      name: "Staging web",
      host: "staging.example.com",
      username: "ubuntu",
      auth: { type: "password" },
      forwards: [forward("st-web", "Web", 8080, 80), forward("st-tls", "", 8443, 443)],
    },
    {
      ...base,
      id: "lab",
      name: "Home lab",
      host: "nas.local",
      username: "admin",
      auth: { type: "agent" },
      forwards: [forward("lab-portainer", "Portainer", 9000, 9000)],
    },
    {
      ...base,
      id: "legacy",
      name: "Legacy box",
      host: "changed.example.com",
      username: "root",
      auth: { type: "password" },
      forwards: [forward("lg-ssh", "", 2222, 22)],
    },
  ];
}

/** What a pretend `~/.ssh/config` holds: one that is here already, two behind a proxy, one without ports. */
function seedSshHosts(): Omit<DiscoveredHost, "alreadyAdded">[] {
  const base = { id: "", port: 22, username: "alice", autoReconnect: true, connectOnLaunch: false };
  return [
    {
      proxy: null,
      profile: {
        ...base,
        name: "web-prod",
        host: "web.example.com",
        port: 2222,
        username: "deploy",
        auth: { type: "key", path: "~/.ssh/id_ed25519" },
        forwards: [forward("", "", 8080, 80), forward("", "", 8443, 443)],
      },
    },
    {
      proxy: null,
      profile: {
        ...base,
        name: "analytics",
        host: "10.0.4.20",
        username: "ubuntu",
        auth: { type: "key", path: "~/.ssh/analytics.pem" },
        forwards: [forward("", "Jupyter", 8888, 8888)],
      },
    },
    {
      // The same account on the same server as "Staging web".
      proxy: null,
      profile: {
        ...base,
        name: "staging",
        host: "staging.example.com",
        username: "ubuntu",
        auth: { type: "agent" },
        forwards: [forward("", "", 8080, 80)],
      },
    },
    {
      proxy: "ProxyJump",
      profile: {
        ...base,
        name: "internal-api",
        host: "api.internal",
        username: "deploy",
        auth: { type: "agent" },
        forwards: [forward("", "", 3000, 3000)],
      },
    },
    {
      proxy: "ProxyCommand",
      profile: {
        ...base,
        name: "build-box",
        host: "build.example.com",
        username: "ci",
        auth: { type: "agent" },
        forwards: [],
      },
    },
    {
      // Nothing to forward in the file: the form asks for the ports.
      proxy: null,
      profile: { ...base, name: "scratch", host: "scratch.example.com", auth: { type: "agent" }, forwards: [] },
    },
  ];
}

/**
 * A pretend core for working on the UI in a plain browser (`npm run dev`): no SSH, but the same
 * states, questions and failures. `window.mock` has levers for provoking the rarer ones.
 */
export class MockBackend implements Backend {
  private profiles = seedProfiles();
  private settings: Settings = { closeToTray: true };
  private readonly runtimes = new Map<ProfileId, ProfileRuntime>();
  private readonly logs: EngineEvent[] = [];
  private readonly open = new Map<string, EngineEvent>();
  private readonly answers = new Map<string, (answer: unknown) => void>();
  private readonly handlers = new Set<(event: EngineEvent) => void>();
  private readonly trusted = new Set(["db.example.com:22"]);
  private readonly saved = new Map<ProfileId, Set<SecretKind>>([["prod", new Set()]]);
  /** Servers whose typed password worked: reconnecting does not ask again until they are switched off. */
  private readonly kept = new Set<ProfileId>();
  private readonly ticks = new Map<ProfileId, number>();
  /** Bumped by every connect and disconnect, so that a superseded attempt notices and stops. */
  private readonly generation = new Map<ProfileId, number>();
  private rev = 0;
  private seq = 0;
  private autostart = false;

  constructor() {
    Object.assign(window, { mock: this });
    for (const profile of this.profiles.filter((p) => p.connectOnLaunch)) void this.connect(profile.id);
  }

  // -- levers for the console ---------------------------------------------------------------

  /** `mock.drop("prod")`: the connection is lost; it comes back after the back-off. */
  drop(id: ProfileId, error: ErrorInfo = { code: "closed", detail: "connection reset by peer" }): void {
    const profile = this.find(id);
    this.stopTraffic(id);
    const generation = this.bump(id);
    this.log(id, "error", error.detail);
    const retryAt = Date.now() + 8000;
    this.set(id, { conn: { state: "reconnecting", attempt: 1, retryAt, error }, connectedAt: null, forwards: this.pending(profile) });
    void sleep(8000).then(() => {
      if (this.generation.get(id) === generation) void this.start(profile, this.bump(id));
    });
  }

  /** `mock.fail("prod", "auth")`: gives up for good. */
  fail(id: ProfileId, code = "auth", detail = "permission denied (publickey)"): void {
    this.stopTraffic(id);
    this.bump(id);
    this.kept.delete(id);
    this.log(id, "error", detail);
    this.set(id, { conn: { state: "failed", error: { code, detail } }, connectedAt: null });
  }

  // -- Backend ------------------------------------------------------------------------------

  async snapshot(): Promise<Snapshot> {
    return {
      profiles: structuredClone(this.profiles),
      settings: { ...this.settings },
      runtimes: structuredClone([...this.runtimes.values()]),
      logs: this.logs.filter((e) => e.type === "log"),
      questions: [...this.open.values()],
    } as Snapshot;
  }

  async onEvent(handler: (event: EngineEvent) => void): Promise<() => void> {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  async saveProfile(input: Profile): Promise<Profile> {
    await sleep(120);
    const profile = structuredClone(input);
    const invalid = (field: string, forwardIndex: number | null = null) =>
      new CommandError({ code: "invalid", field, forwardIndex, detail: `${field} is not valid` });
    profile.host = profile.host.trim();
    profile.username = profile.username.trim();
    if (!profile.host) throw invalid("host");
    if (!profile.username) throw invalid("username");
    if (profile.auth.type === "key" && !profile.auth.path.trim()) throw invalid("keyPath");
    if (profile.forwards.length === 0) throw invalid("forwards");
    profile.forwards.forEach((f, i) => {
      if (!f.localPort) throw invalid("localPort", i);
      if (!f.remotePort) throw invalid("remotePort", i);
      if (profile.forwards.findIndex((o) => o.localPort === f.localPort) < i) throw invalid("duplicatePort", i);
      f.id ||= crypto.randomUUID();
      f.bindHost ||= "127.0.0.1";
      f.remoteHost ||= "127.0.0.1";
    });
    profile.name = profile.name.trim() || profile.host;
    profile.id ||= crypto.randomUUID();
    const index = this.profiles.findIndex((p) => p.id === profile.id);
    if (index >= 0) this.profiles[index] = profile;
    else this.profiles.push(profile);
    if (this.isActive(profile.id)) {
      await this.disconnect(profile.id);
      void this.connect(profile.id);
    }
    return structuredClone(profile);
  }

  async deleteProfile(id: ProfileId): Promise<void> {
    this.find(id);
    await this.disconnect(id);
    this.profiles = this.profiles.filter((p) => p.id !== id);
    this.runtimes.delete(id);
  }

  async discoverSshHosts(): Promise<DiscoveredHost[]> {
    await sleep(200);
    return seedSshHosts().map((host) => ({
      ...host,
      alreadyAdded: this.profiles.some(
        (p) =>
          p.host.toLowerCase() === host.profile.host.toLowerCase() &&
          p.port === host.profile.port &&
          p.username === host.profile.username,
      ),
    }));
  }

  async connect(id: ProfileId): Promise<void> {
    const profile = this.find(id);
    if (this.isActive(id)) return;
    void this.start(profile, this.bump(id));
  }

  async disconnect(id: ProfileId): Promise<void> {
    this.find(id);
    this.bump(id);
    this.stopTraffic(id);
    this.kept.delete(id);
    if (this.runtimes.has(id)) {
      this.log(id, "info", "disconnected");
      this.set(id, { conn: { state: "idle" }, forwards: [], connectedAt: null });
    }
  }

  async answerHostKey(requestId: string, trust: boolean): Promise<boolean> {
    return this.answer(requestId, trust);
  }

  async answerSecret(requestId: string, value: string | null, remember: boolean): Promise<boolean> {
    return this.answer(requestId, value === null ? null : { value, remember });
  }

  async savedSecrets(id: ProfileId): Promise<SecretKind[]> {
    return [...(this.saved.get(id) ?? [])];
  }

  async forgetSecrets(id: ProfileId): Promise<void> {
    this.saved.delete(id);
  }

  async forgetHostKey(id: ProfileId): Promise<boolean> {
    const profile = this.find(id);
    return this.trusted.delete(`${profile.host}:${profile.port}`);
  }

  async setSettings(settings: Settings): Promise<void> {
    this.settings = { ...settings };
  }

  async autostartEnabled(): Promise<boolean> {
    return this.autostart;
  }

  async setAutostart(enabled: boolean): Promise<void> {
    this.autostart = enabled;
  }

  async setTrayLabels(_labels: TrayLabels): Promise<void> {}

  async pickKeyFile(): Promise<string | null> {
    await sleep(250);
    return "~/.ssh/id_rsa";
  }

  async uiReady(): Promise<void> {}
  async quit(): Promise<void> {}

  // -- the pretend engine -------------------------------------------------------------------

  private async start(profile: Profile, generation: number): Promise<void> {
    const id = profile.id;
    const current = () => this.generation.get(id) === generation;
    this.set(id, { conn: { state: "connecting" }, forwards: this.pending(profile), connectedAt: null });
    this.log(id, "info", `connecting to ${profile.host}:${profile.port}`);
    await sleep(900);
    if (!current()) return;

    if (profile.host.startsWith("changed.")) {
      return this.giveUp(id, "hostKeyChanged", "host key changed: on file SHA256:Qm9v…, presented SHA256:7Yk2…");
    }
    const place = `${profile.host}:${profile.port}`;
    if (!this.trusted.has(place)) {
      const trusted = await this.ask<boolean>({
        type: "hostKey",
        requestId: crypto.randomUUID(),
        profileId: id,
        profileName: profile.name,
        host: profile.host,
        port: profile.port,
        keyType: "ssh-ed25519",
        fingerprint: "SHA256:uNiVeRsAlLyUnIqUeFiNgErPrInTfOrThEmOcKsErVeR0123",
        otherKnown: false,
      });
      if (!current()) return;
      if (!trusted) return this.giveUp(id, "hostKeyRejected", "the host key was not trusted");
      this.trusted.add(place);
    }

    if (profile.auth.type !== "agent" && !(await this.signIn(profile, current))) return;
    if (!current()) return;

    this.log(id, "info", `logged in as ${profile.username}`);
    const forwards = profile.forwards.map((f) => this.listening(f));
    this.set(id, { conn: { state: "connected" }, forwards, connectedAt: Date.now() });
    this.ticks.set(id, window.setInterval(() => this.traffic(id), 500));
  }

  /** Asks for the password up to three times. Returns whether it was right. */
  private async signIn(profile: Profile, current: () => boolean): Promise<boolean> {
    const kind: SecretKind = profile.auth.type === "key" ? "passphrase" : "password";
    if (this.saved.get(profile.id)?.has(kind) || this.kept.has(profile.id)) return true;
    for (let attempt = 0; attempt < 3; attempt++) {
      const reply = await this.ask<{ value: string; remember: boolean } | null>({
        type: "secret",
        requestId: crypto.randomUUID(),
        profileId: profile.id,
        profileName: profile.name,
        kind,
        retry: attempt > 0,
      });
      if (!current()) return false;
      if (reply === null) {
        this.giveUp(profile.id, "cancelled", "cancelled");
        return false;
      }
      if (reply.value === MOCK_PASSWORD) {
        this.kept.add(profile.id);
        if (reply.remember) this.saved.set(profile.id, new Set([kind]));
        return true;
      }
    }
    this.giveUp(profile.id, "auth", "permission denied");
    return false;
  }

  private giveUp(id: ProfileId, code: string, detail: string): void {
    this.kept.delete(id);
    this.log(id, "error", detail);
    this.set(id, { conn: { state: "failed", error: { code, detail } }, forwards: [], connectedAt: null });
  }

  private ask<T>(question: Extract<EngineEvent, { type: "hostKey" | "secret" }>): Promise<T> {
    return new Promise<T>((resolve) => {
      this.open.set(question.requestId, question);
      this.answers.set(question.requestId, resolve as (answer: unknown) => void);
      this.emit(question);
    });
  }

  private answer(requestId: string, answer: unknown): boolean {
    const resolve = this.answers.get(requestId);
    if (!resolve) return false;
    this.answers.delete(requestId);
    this.open.delete(requestId);
    this.emit({ type: "promptClosed", requestId });
    resolve(answer);
    return true;
  }

  private traffic(id: ProfileId): void {
    const runtime = this.runtimes.get(id);
    if (!runtime) return;
    const busy = Math.random() < 0.8;
    const forwards = runtime.forwards.map((f, index) => {
      if (f.state.state !== "listening" || !busy || index > 0 && Math.random() < 0.5) return { ...f, active: 0 };
      const up = Math.floor(Math.random() * 6000);
      const down = Math.floor(Math.random() * 90000);
      return {
        ...f,
        active: 1 + Math.floor(Math.random() * 3),
        total: f.total + (Math.random() < 0.3 ? 1 : 0),
        bytesUp: f.bytesUp + up,
        bytesDown: f.bytesDown + down,
      };
    });
    this.set(id, { forwards });
  }

  private pending(profile: Profile): ForwardRuntime[] {
    return profile.forwards.map((f) => ({ id: f.id, state: { state: "pending" }, active: 0, total: 0, bytesUp: 0, bytesDown: 0 }));
  }

  private listening(f: Forward): ForwardRuntime {
    return { id: f.id, state: { state: "listening" }, active: 0, total: 0, bytesUp: 0, bytesDown: 0 };
  }

  // -- bookkeeping --------------------------------------------------------------------------

  private find(id: ProfileId): Profile {
    const profile = this.profiles.find((p) => p.id === id);
    if (!profile) throw new CommandError({ code: "notFound" });
    return profile;
  }

  private isActive(id: ProfileId): boolean {
    const state = this.runtimes.get(id)?.conn.state;
    return state === "connecting" || state === "connected" || state === "reconnecting";
  }

  private bump(id: ProfileId): number {
    const next = (this.generation.get(id) ?? 0) + 1;
    this.generation.set(id, next);
    return next;
  }

  private stopTraffic(id: ProfileId): void {
    window.clearInterval(this.ticks.get(id));
    this.ticks.delete(id);
  }

  private set(id: ProfileId, change: Partial<ProfileRuntime>): void {
    const before = this.runtimes.get(id) ?? { profileId: id, conn: { state: "idle" }, forwards: [], connectedAt: null, rev: 0 };
    const next: ProfileRuntime = { ...before, ...change, rev: ++this.rev };
    this.runtimes.set(id, next);
    this.emit({ type: "state", ...next });
  }

  private log(profileId: ProfileId, level: LogLevel, message: string): void {
    const line: EngineEvent = { type: "log", seq: ++this.seq, profileId, ts: Date.now(), level, message };
    this.logs.push(line);
    this.emit(line);
  }

  private emit(event: EngineEvent): void {
    setTimeout(() => this.handlers.forEach((handler) => handler(structuredClone(event))), 0);
  }
}
