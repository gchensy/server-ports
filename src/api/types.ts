// The JSON the Rust core speaks (src-tauri/src/model.rs and backend.rs). Keep the two in step.

export type ProfileId = string;

export type Auth =
  | { type: "password" }
  | { type: "key"; path: string }
  | { type: "agent" };

export interface Forward {
  id: string;
  label: string;
  bindHost: string;
  localPort: number;
  remoteHost: string;
  remotePort: number;
}

export interface Profile {
  id: ProfileId;
  name: string;
  host: string;
  port: number;
  username: string;
  auth: Auth;
  forwards: Forward[];
  autoReconnect: boolean;
  connectOnLaunch: boolean;
}

export interface Settings {
  closeToTray: boolean;
}

/** A failure the UI can localise: `code` is stable, `detail` is raw technical text. */
export interface ErrorInfo {
  code: string;
  detail: string;
}

export type ConnState =
  | { state: "idle" }
  | { state: "connecting" }
  | { state: "connected" }
  | { state: "reconnecting"; attempt: number; retryAt: number; error: ErrorInfo }
  | { state: "failed"; error: ErrorInfo };

export type ForwardState =
  | { state: "pending" }
  | { state: "listening" }
  | { state: "failed"; error: ErrorInfo };

export interface ForwardRuntime {
  id: string;
  state: ForwardState;
  active: number;
  total: number;
  bytesUp: number;
  bytesDown: number;
}

export interface ProfileRuntime {
  profileId: ProfileId;
  conn: ConnState;
  forwards: ForwardRuntime[];
  connectedAt: number | null;
  /** Grows with every change; of two copies the higher one is newer. */
  rev: number;
}

export type LogLevel = "info" | "warn" | "error";

export interface LogLine {
  /** Position in the log of the whole app. */
  seq: number;
  profileId: ProfileId;
  ts: number;
  level: LogLevel;
  message: string;
}

export type SecretKind = "password" | "passphrase";

export interface HostKeyRequest {
  requestId: string;
  profileId: ProfileId;
  profileName: string;
  host: string;
  port: number;
  keyType: string;
  fingerprint: string;
  /** The server is already known by a key of another type. */
  otherKnown: boolean;
}

export interface SecretRequest {
  requestId: string;
  profileId: ProfileId;
  profileName: string;
  kind: SecretKind;
  /** The previous attempt was rejected. */
  retry: boolean;
}

export type StateEvent = { type: "state" } & ProfileRuntime;
export type LogEvent = { type: "log" } & LogLine;
export type HostKeyEvent = { type: "hostKey" } & HostKeyRequest;
export type SecretEvent = { type: "secret" } & SecretRequest;

/** Everything the engine reports, on one channel. */
export type EngineEvent =
  | StateEvent
  | LogEvent
  | HostKeyEvent
  | SecretEvent
  | { type: "promptClosed"; requestId: string };

/** A question waiting for the user. */
export type Prompt = HostKeyEvent | SecretEvent;

export interface Snapshot {
  profiles: Profile[];
  settings: Settings;
  runtimes: ProfileRuntime[];
  logs: LogLine[];
  /** The `hostKey` and `secret` events whose answer is still awaited. */
  questions: EngineEvent[];
}

/** What a command can fail with. `field` is the code of the rule a profile broke. */
export type BackendError =
  | { code: "invalid"; field: string; forwardIndex: number | null; detail: string }
  | { code: "notFound" }
  | { code: "storage"; detail: string }
  | { code: "keychain"; detail: string };

export interface TrayLabels {
  show: string;
  noProfiles: string;
  connectAll: string;
  disconnectAll: string;
  quit: string;
  /** `{n}` stands for the number of connected profiles. */
  tooltip: string;
}
