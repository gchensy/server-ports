<div align="center">

# Server Ports

**Map remote server ports to your local machine over SSH.**

English | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **Status: early development.** There is no release yet. The tunnel engine and the interface are in place, but the app has only been built and run on Linux so far; macOS and Windows are untested.

Server Ports is a small cross-platform desktop app (Windows, macOS, Linux) that keeps your SSH port forwards running. Pick a server, list the ports you need, and they appear on `localhost`. It is the same thing as `ssh -L`, minus the terminal juggling.

## Features

- **Server profiles**: one profile per SSH server, with as many port mappings as you need.
- **Three ways to sign in**: password, private key file (with an optional passphrase), or ssh-agent (OpenSSH agent or Pageant).
- **Secrets stay out of the config file**: if you ask for it, passwords and key passphrases are kept in the macOS Keychain, Windows Credential Manager or the Linux Secret Service; otherwise they are only held in memory while a server is connected.
- **Host key verification**: a server key you have not seen before is shown with its fingerprint and trusted only if you say so; a key that has changed is flagged.
- **Private by default**: forwards listen on `127.0.0.1`, so only your own machine can reach them.
- **Auto-reconnect**: dropped connections are retried with a growing delay (from 1 s up to 30 s), and a profile can connect as soon as the app starts.
- **Live status**: connection state per server, listening state per mapping, open connections, traffic counters and a small throughput graph.
- **Lives in the tray**: closing the window hides the app in the system tray instead of quitting (this can be turned off), and the tray menu switches each server on or off. The app can also start when you sign in.
- **Light and dark theme**, with an interface in English or Simplified Chinese.
- **Small footprint**: built with Tauri 2, a Rust core and the system web view, with no bundled browser.

## How it works

Every mapping forwards a port on your machine to an address that the SSH server can reach:

```text
127.0.0.1:8080 (your machine)  ──encrypted SSH──►  127.0.0.1:80 (the server)
```

That is the same as `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. The target is resolved on the server, so `127.0.0.1` means the server itself, and any other host the server can reach works too.

## Known limitations

- No keyboard-interactive (two-factor) login, jump hosts (`ProxyJump`) or host certificates yet.
- Only local forwards (`ssh -L`); remote (`-R`) and SOCKS (`-D`) forwards are not supported.
- Servers are added by hand; `~/.ssh/config` is not read.
- On GNOME the tray icon needs the AppIndicator extension. Without it, turn off "Keep running in the tray when the window is closed" in Settings, or start the app again to bring its window back.

## Development

Prerequisites:

- Node.js (current LTS) and npm
- Rust 1.89 or newer ([rustup](https://rustup.rs))
- Tauri's system dependencies for your OS, see the [Tauri prerequisites](https://tauri.app/start/prerequisites/). On Debian/Ubuntu:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Then:

```bash
npm install
npm run tauri dev    # run the app with hot reload
npm run dev          # the interface alone in a browser, with a fake backend instead of SSH
npm run tauri build  # build installers into src-tauri/target/release/bundle/
npm run typecheck    # type-check the frontend
cargo test --manifest-path src-tauri/Cargo.toml  # run the Rust tests
```

## Project layout

```text
src/         React + TypeScript user interface
src-tauri/   Rust core: SSH port forwarding, profile storage, OS credential store, tray
```
