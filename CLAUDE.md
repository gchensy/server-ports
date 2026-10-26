# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Server Ports** is a cross-platform (Windows / macOS / Linux) desktop app that maps ports of a remote server to the local machine over SSH (local port forwarding, like `ssh -L`). Tauri 2 shell, React + TypeScript UI, Rust core built on `russh`.

Identifiers: npm package and crate `server-ports` (lib `server_ports_lib`), bundle id `com.chensy.server-ports`.

Status (2026-10-26): early development. The Rust core (model, stores, secrets, prompts, the `ssh/` engine and the `Backend` facade), the Tauri layer (commands, tray, close-to-tray, autostart, single instance) and the React UI (nine languages, light and dark) all exist and run on Linux. `cargo test` passes against an in-process SSH server, the UI was checked in a browser against the mock backend, and the real app starts, shows its tray icon and loads the UI. Never built on macOS or Windows (the Windows branch of `connect_agent` is not compiled anywhere yet), and nothing is released. GitHub Actions workflows for CI and for building the installers exist but have not run yet. Known gaps: no keyboard-interactive (2FA) login, ProxyJump, host certificates, `-R` / `-D` forwards or `~/.ssh/config` import. Update this line as that changes.

## Commands

Run from the repo root.

| Task | Command |
|---|---|
| Install JS dependencies | `npm install` |
| Run the app (Vite + Tauri, hot reload) | `npm run tauri dev` |
| Frontend only | `npm run dev` (port 1420). Outside the desktop shell the UI runs against `MockBackend` (dev builds only) with sample servers; the password is `hunter2`, and `window.mock.drop(id)` / `.fail(id)` in the console provoke a lost connection or a failure |
| Type-check the frontend | `npm run typecheck` |
| Build the frontend | `npm run build` (`tsc && vite build`) |
| Build installers | `npm run tauri build` (output in `src-tauri/target/release/bundle/`) |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Rust lint / format | `cargo clippy --manifest-path src-tauri/Cargo.toml` / `cargo fmt --manifest-path src-tauri/Cargo.toml` |
| Regenerate the app icons | `npm run tauri icon src-tauri/icons/icon.svg`, then delete the `android/` and `ios/` folders and `64x64.png` it adds (desktop only) |

- Rust comes from rustup in `~/.cargo/bin`. If `cargo` is not found in a non-login shell, run `. "$HOME/.cargo/env"` first. The minimum Rust version is 1.89 (edition 2024).
- On Debian/Ubuntu the Tauri build needs system libraries (webkit2gtk 4.1, libsoup 3, librsvg, libxdo, ...); see https://tauri.app/start/prerequisites/.
- There is no JS test runner yet. Rust unit tests sit next to the code in `#[cfg(test)] mod tests`.

## Layout

- `src/`: React UI. TypeScript is strict, and `noUnusedLocals` / `noUnusedParameters` are on. Services are classes (object-oriented on purpose), read from components through hooks.
  - `api/`: the `Backend` interface, `TauriBackend` (IPC), `MockBackend` (browser development only, tree-shaken from production), the wire types in `types.ts` (keep them in step with `model.rs` and `backend.rs`) and `errors.ts`.
  - `state/`: `AppStore` (all UI state, read through `useSyncExternalStore`; merges the startup snapshot and the event stream by `rev` and `seq`), `ThroughputTracker`, `Appearance` (light, dark, system), `prefs.ts` (localStorage), `services.tsx` (context and hooks).
  - `i18n/`: the `I18n` class and one typed catalogue per language (`en`, `zh-CN`, `zh-TW`, `ja`, `ko`, `es`, `fr`, `de`, `ru`). `en.ts` is the source; the others are typed `Catalog` and have to define the same keys (the compiler checks). To add a language: a catalogue file, then `LANGUAGE_NAMES` and `catalogs` in `index.ts`, a font line in `tokens.css` if it is a CJK language, and the READMEs.
  - `components/`, `styles/` (plain CSS on the variables in `tokens.css`, no CSS framework).
- `src-tauri/`: Rust crate and Tauri config. `tauri.conf.json` holds the window, CSP and bundle settings. `capabilities/default.json` lists what the webview may call, so add the permission there when you add a plugin that needs one (the app's own commands are allowed by default). `icons/icon.svg` is the source of the icon set.
- `src-tauri/src/`: the core knows nothing about Tauri or the UI; it reports through an event sink.
  - `model.rs`: persisted config (`Profile`, `Forward`, `Auth`, `Settings`), runtime state (`ProfileRuntime`, `ConnState`, `ForwardState`), prompts, and the `Event` enum.
  - `events.rs`: the `EventSink` trait, with `NullSink` and `ChannelSink` (tests).
  - `interaction.rs`: questions put to the human (trust this host key? what is the password?) as awaited requests answered through `answer_*`, with a 90 s timeout. Open questions are remembered, so a window that opens late can still show them.
  - `known_hosts.rs`: trust-on-first-use host key store (`known_hosts.json`).
  - `secrets.rs`: passwords and key passphrases in the OS credential store via `keyring`.
  - `store.rs`: `profiles.json` persistence.
  - `backend.rs`: the `Backend` facade over the store, the tunnels, the secrets, the settings and the servers found in `~/.ssh/config`. Every Tauri command is one call on it, and it is where behaviour is tested.
  - `ssh_config.rs`: reads `~/.ssh/config` and its `Include`s (read-only) into `SshHost`s: a not yet saved `Profile` per `Host` name, plus the `Proxy` it needs, if any. `LocalUser` is whose home and name the `~` and `%d` / `%u` refer to.
  - `ssh/`: the tunnel engine: `TunnelManager` (`manager.rs`), one `Tunnel` per running profile (`tunnel.rs`: connect, serve, back off, reconnect), a `Session` per authenticated SSH connection (`session.rs`), a `Forwarder` per local listener (`forward.rs`; every accepted socket becomes a `direct-tcpip` channel), `Authenticator` (`auth.rs`), `ClientHandler` (`handler.rs`, host key checks), `Env` / `Reporter` / `StatusBoard` (`env.rs`, shared state and recent log lines) and `TunnelError` (`error.rs`).
  - `app.rs` (plugins, setup, window and exit events, the sink that forwards events to the webview), `commands.rs` (thin Tauri commands) and `tray.rs` (tray icon and menu) are the only files that know Tauri.
  - `testing.rs` (tests only): an in-process russh server, an echo target and helpers shared by the tests.
  - `util.rs`: `now_ms`, `new_id`, `expand_tilde`, `write_atomic`.
- `.github/workflows/`: `ci.yml` and `release.yml`, see "CI and releases".

Keep this list in sync when you add or move modules.

## Conventions that are not obvious from the code

- **IPC is camelCase JSON.** Everything that crosses the Tauri boundary uses `#[serde(rename_all = "camelCase")]`. Tagged enums use `type` (`Event`, `Auth`) or `state` (`ConnState`, `ForwardState`).
- **One event channel.** The engine emits the single `Event` enum and never calls Tauri directly, which keeps it testable with `ChannelSink`.
- **Errors carry a stable `code`.** `ErrorInfo { code, detail }`, `ValidationError.code` and `TunnelError::code()` are what the UI keys its localised messages on; `detail` is raw technical text shown under that headline. Do not put user-facing sentences in Rust, and do not rename codes casually.
- **Retry policy lives in `TunnelError::retryable()`.** Only transient failures (network, timeout, closed session) are retried. Auth failures, rejected or changed host keys and unavailable local ports are not.
- **Host keys:** an unseen key asks the user. A different key of an already-known key type is reported as `HostKeyCheck::Changed` (a possible man-in-the-middle), surfaced as `TunnelError::HostKeyChanged` and never retried. Server host certificates are not supported.
- **Secrets never touch `profiles.json`.** They go to the OS credential store (`secrets.rs`), and only if the user ticks "remember". The keyring calls block, so they run through `spawn_blocking`. A typed secret that worked is also kept in memory while its tunnel runs (`KeptSecrets` in `ssh/auth.rs`), so an automatic reconnect does not ask again; a kept secret the server turns down is dropped and the user is asked.
- **Files are written atomically** (`util::write_atomic`), and a corrupt `profiles.json` is moved to `profiles.json.corrupt` instead of being overwritten. Stored data must keep loading after the format grows, so new fields need `#[serde(default)]`.
- **Forwards bind to `127.0.0.1` by default.** Any other bind address is a deliberate choice. Duplicate local ports are rejected in `Profile::normalize`.
- **Dependency choices are deliberate.** `russh` uses the `ring` backend instead of `aws-lc-rs` so that all three OSes build in CI without cmake or NASM. The release profile keeps `panic = "unwind"` so a panicking tunnel task cannot take the resident app down. Read the comments in `src-tauri/Cargo.toml` before changing either.
- **The Vite dev server is pinned to port 1420** (`strictPort`) and ignores `src-tauri/` in its watcher.
- **The window is created hidden** and shown when the UI calls `ui_ready` (or after 5 s). A hidden page paints no frames, so the UI must not wait for `requestAnimationFrame` before calling it. Starting with `--hidden` (the autostart entry) keeps it hidden.
- **Tauri's npm packages and Rust crates must share their major.minor version** (`tauri dev` complains otherwise); the `@tauri-apps/*` packages are pinned with `~` for that reason.
- **Languages:** the tray labels go to Rust in the current language, and the tooltip is a template whose `{n}` Rust fills in, so its wording has to work for any number (no plural forms there). Plurals elsewhere come from `Intl.PluralRules` (`ports.one` / `.other`, plus `.few` / `.many` where a language needs them); `duration.*` hold the whole text with `{n}`, so a language can add its own space or unit. Each CJK language lists its own fonts in `tokens.css` (`:root:lang(...)`), because the same Han characters are drawn differently per language.
- **UI details that are deliberate:** addresses are shown as `127.0.0.1:port` rather than `localhost` (which may resolve to `::1` first); CSS avoids `light-dark()`, `color-mix()` and transitions of the `d` attribute because the oldest supported WebKitGTK and WKWebView lack them; the dark colour tokens appear twice in `tokens.css` (media query and explicit theme), keep them equal.

## CI and releases

- `ci.yml` runs on pull requests and on `main`: the frontend (`npm ci`, typecheck, build) and the Rust side on Linux, macOS and Windows (`cargo fmt --check` on Linux only, `cargo clippy --all-targets -- -D warnings`, `cargo test`). It is the only place the macOS and Windows code is compiled.
- `release.yml` runs on a tag `v*`. By hand it only builds and keeps the installers as workflow artifacts. `tauri-apps/tauri-action` builds `.deb` / `.rpm` / `.AppImage` (ubuntu-22.04), `.dmg` for aarch64 and x86_64, and `.exe` / `.msi`, and attaches them to a draft release. It first checks that the tag equals the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, so bump all three together.
- The builds are unsigned: macOS is signed ad hoc (`APPLE_SIGNING_IDENTITY: "-"` in the workflow), Windows is not signed. Signing and notarization need certificates; they go into GitHub secrets (see Tauri's signing guides), never into the repo.
- Written against the action versions current on 2026-10-26 (`checkout@v7`, `setup-node@v7`, `tauri-action@v1`); the workflows have not run yet, so expect the first run on macOS and Windows to turn up platform problems.

## Documentation

`README.md` is the English source of truth. The translations are `README.<lang>.md` in the repo root (zh-CN, zh-TW, ja, ko, es, fr, de, ru). They share the same structure, and each starts with the same language switcher line. When you change `README.md`, update every translation in the same commit. Commands, code blocks and file names stay untranslated.

## Git and GitHub

- Remote: `github.com/gchensy/server-ports` (private), default branch `main`.
- `gh` on this machine is logged in to two accounts: `gchensy`, which owns this repo, and `CaoChensy`. Run `gh auth status` before pushing or opening a PR, and `gh auth switch --user gchensy` if the wrong account is active.
- Several Claude sessions may work in this checkout at the same time. Check `git status` before committing, stage only your own files, and do not revert changes you did not make.
- Never commit `node_modules/`, `dist/`, `src-tauri/target/` or `src-tauri/gen/schemas/` (all ignored already), or anything that holds credentials or private keys.
