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

/** Everything the UI asks of the Rust core. Failures are thrown as `CommandError`. */
export interface Backend {
  snapshot(): Promise<Snapshot>;
  /** Resolves once the listener is in place; call the result to stop listening. */
  onEvent(handler: (event: EngineEvent) => void): Promise<() => void>;

  saveProfile(profile: Profile): Promise<Profile>;
  deleteProfile(id: ProfileId): Promise<void>;
  /** The servers in the user's `~/.ssh/config`. Nothing is saved. */
  discoverSshHosts(): Promise<DiscoveredHost[]>;
  connect(id: ProfileId): Promise<void>;
  disconnect(id: ProfileId): Promise<void>;

  /** `false`: the question was gone already. */
  answerHostKey(requestId: string, trust: boolean): Promise<boolean>;
  /** `value === null` cancels the question. */
  answerSecret(requestId: string, value: string | null, remember: boolean): Promise<boolean>;

  savedSecrets(id: ProfileId): Promise<SecretKind[]>;
  forgetSecrets(id: ProfileId): Promise<void>;
  forgetHostKey(id: ProfileId): Promise<boolean>;

  setSettings(settings: Settings): Promise<void>;
  autostartEnabled(): Promise<boolean>;
  setAutostart(enabled: boolean): Promise<void>;
  setTrayLabels(labels: TrayLabels): Promise<void>;

  /** Asks for a private key file; `null` if the user backs out. */
  pickKeyFile(): Promise<string | null>;
  /** Opens a web link in the default browser. Only addresses of this computer are allowed. */
  openUrl(url: string): Promise<void>;
  /** The first frame is painted: the window may appear. */
  uiReady(): Promise<void>;
  quit(): Promise<void>;
}
