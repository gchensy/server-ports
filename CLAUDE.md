# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Server Ports** is a cross-platform (Windows / macOS / Linux) desktop app that maps ports of a remote server to the local machine over SSH (local port forwarding, like `ssh -L`). Tauri 2 shell, React + TypeScript UI, Rust core built on `russh`.

Identifiers: npm package and crate `server-ports` (lib `server_ports_lib`), bundle id `com.chensy.server-ports`.

Status (2026-10-25): early development. The Rust core is being written first: the data model, the profile and known-hosts stores, credential-store access, the prompt plumbing and the error types exist. The tunnel engine, the Tauri commands, the tray and the real UI do not yet (`lib.rs` still runs a bare Tauri builder and the React UI is the stock create-tauri-app template). Update this line as that changes.

## Commands

Run from the repo root.

| Task | Command |
|---|---|
| Install JS dependencies | `npm install` |
| Run the app (Vite + Tauri, hot reload) | `npm run tauri dev` |
| Frontend only | `npm run dev` (port 1420; Tauri APIs such as `invoke` do not work in a plain browser) |
| Type-check the frontend | `npm run typecheck` |
| Build the frontend | `npm run build` (`tsc && vite build`) |
| Build installers | `npm run tauri build` (output in `src-tauri/target/release/bundle/`) |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Rust lint / format | `cargo clippy --manifest-path src-tauri/Cargo.toml` / `cargo fmt --manifest-path src-tauri/Cargo.toml` |

- Rust comes from rustup in `~/.cargo/bin`. If `cargo` is not found in a non-login shell, run `. "$HOME/.cargo/env"` first. The minimum Rust version is 1.89 (edition 2024).
- On Debian/Ubuntu the Tauri build needs system libraries (webkit2gtk 4.1, libsoup 3, librsvg, libxdo, ...); see https://tauri.app/start/prerequisites/.
- There is no JS test runner yet. Rust unit tests sit next to the code in `#[cfg(test)] mod tests`.

## Layout

- `src/`: React UI. TypeScript is strict, and `noUnusedLocals` / `noUnusedParameters` are on.
- `src-tauri/`: Rust crate and Tauri config. `tauri.conf.json` holds the window and bundle settings. `capabilities/default.json` lists what the webview may call, so add the permission there when you add a plugin or command that needs one.
- `src-tauri/src/`: the core knows nothing about Tauri or the UI; it reports through an event sink.
  - `model.rs`: persisted config (`Profile`, `Forward`, `Auth`, `Settings`), runtime state (`ProfileRuntime`, `ConnState`, `ForwardState`), prompts, and the `Event` enum.
  - `events.rs`: the `EventSink` trait, with `NullSink` and `ChannelSink` (tests). The Tauri layer is meant to plug in a sink that forwards events to the webview.
  - `interaction.rs`: questions put to the human (trust this host key? what is the password?) as awaited requests answered through `answer_*`, with a 90 s timeout.
  - `known_hosts.rs`: trust-on-first-use host key store (`known_hosts.json`).
  - `secrets.rs`: passwords and key passphrases in the OS credential store via `keyring`.
  - `store.rs`: `profiles.json` persistence.
  - `ssh/`: the tunnel engine. The shape described in `ssh/mod.rs` is a `TunnelManager` facade, one `Tunnel` per running profile (connect, serve, back off, reconnect), a `Session` per authenticated SSH connection and a `Forwarder` per local listener, where every accepted socket becomes a `direct-tcpip` channel. So far only `state.rs` (shared runtime state and recent log lines) and `error.rs` (`TunnelError`) exist.
  - `util.rs`: `now_ms`, `new_id`, `expand_tilde`, `write_atomic`.

Keep this list in sync when you add or move modules.

## Conventions that are not obvious from the code

- **IPC is camelCase JSON.** Everything that crosses the Tauri boundary uses `#[serde(rename_all = "camelCase")]`. Tagged enums use `type` (`Event`, `Auth`) or `state` (`ConnState`, `ForwardState`).
- **One event channel.** The engine emits the single `Event` enum and never calls Tauri directly, which keeps it testable with `ChannelSink`.
- **Errors carry a stable `code`.** `ErrorInfo { code, detail }`, `ValidationError.code` and `TunnelError::code()` are what the UI keys its localised messages on; `detail` is raw technical text shown under that headline. Do not put user-facing sentences in Rust, and do not rename codes casually.
- **Retry policy lives in `TunnelError::retryable()`.** Only transient failures (network, timeout, closed session) are retried. Auth failures, rejected or changed host keys and unavailable local ports are not.
- **Host keys:** an unseen key asks the user. A different key of an already-known key type is reported as `HostKeyCheck::Changed` (a possible man-in-the-middle), surfaced as `TunnelError::HostKeyChanged` and never retried. Server host certificates are not supported.
- **Secrets never touch `profiles.json`.** They go to the OS credential store (`secrets.rs`). The keyring calls block, so they run through `spawn_blocking`.
- **Files are written atomically** (`util::write_atomic`), and a corrupt `profiles.json` is moved to `profiles.json.corrupt` instead of being overwritten. Stored data must keep loading after the format grows, so new fields need `#[serde(default)]`.
- **Forwards bind to `127.0.0.1` by default.** Any other bind address is a deliberate choice. Duplicate local ports are rejected in `Profile::normalize`.
- **Dependency choices are deliberate.** `russh` uses the `ring` backend instead of `aws-lc-rs` so that all three OSes build in CI without cmake or NASM. The release profile keeps `panic = "unwind"` so a panicking tunnel task cannot take the resident app down. Read the comments in `src-tauri/Cargo.toml` before changing either.
- **The Vite dev server is pinned to port 1420** (`strictPort`) and ignores `src-tauri/` in its watcher.

## Documentation

`README.md` is the English source of truth. The translations are `README.<lang>.md` in the repo root (zh-CN, zh-TW, ja, ko, es, fr, de, ru). They share the same structure, and each starts with the same language switcher line. When you change `README.md`, update every translation in the same commit. Commands, code blocks and file names stay untranslated.

## Git and GitHub

- Remote: `github.com/gchensy/server-ports` (private), default branch `main`.
- `gh` on this machine is logged in to two accounts: `gchensy`, which owns this repo, and `CaoChensy`. Run `gh auth status` before pushing or opening a PR, and `gh auth switch --user gchensy` if the wrong account is active.
- Several Claude sessions may work in this checkout at the same time. Check `git status` before committing, stage only your own files, and do not revert changes you did not make.
- Never commit `node_modules/`, `dist/`, `src-tauri/target/` or `src-tauri/gen/schemas/` (all ignored already), or anything that holds credentials or private keys.
