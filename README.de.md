<div align="center">

# Server Ports

**Ordnet Ports entfernter Server über SSH Ihrem lokalen Rechner zu.**

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | Deutsch | [Русский](README.ru.md)

</div>

> **Status: frühe Entwicklung.** Es gibt noch keine veröffentlichte Version. Diese README beschreibt, was die App leisten soll; einiges davon ist noch in Arbeit.

Server Ports ist eine kleine plattformübergreifende Desktop-App (Windows, macOS, Linux), die Ihre SSH-Portweiterleitungen am Laufen hält. Server auswählen, benötigte Ports eintragen, und sie erscheinen auf `localhost`. Das ist dasselbe wie `ssh -L`, nur ohne Terminal-Gefummel.

## Funktionen

- **Serverprofile**: ein Profil pro SSH-Server, mit beliebig vielen Port-Zuordnungen.
- **Drei Anmeldearten**: Passwort, Datei mit privatem Schlüssel (optional mit Passphrase) oder ssh-agent (OpenSSH-Agent oder Pageant).
- **Geheimnisse bleiben im Anmeldedatenspeicher des Betriebssystems**: Passwörter und Schlüssel-Passphrasen liegen im macOS-Schlüsselbund, in der Windows-Anmeldeinformationsverwaltung oder im Linux Secret Service, nie in der Konfigurationsdatei.
- **Überprüfung des Host-Schlüssels**: Der Schlüssel eines bislang unbekannten Servers wird mit seinem Fingerabdruck angezeigt und nur akzeptiert, wenn Sie das bestätigen; ein geänderter Schlüssel wird gemeldet.
- **Standardmäßig privat**: Weiterleitungen lauschen auf `127.0.0.1`, sodass nur Ihr eigener Rechner sie erreichen kann.
- **Automatische Wiederverbindung**: Abgebrochene Verbindungen werden automatisch erneut versucht; ein Profil kann sich außerdem direkt beim Start der App verbinden.
- **Live-Status**: Verbindungsstatus pro Server, Listening-Status pro Zuordnung, offene Verbindungen und Datenverkehrszähler.
- **Läuft im Tray**: Beim Schließen des Fensters wird die App im Infobereich der Taskleiste ausgeblendet, statt beendet zu werden (abschaltbar).
- **Schlank**: Mit Tauri 2 gebaut, mit Rust-Kern und der System-Webview, ohne mitgelieferten Browser.

## So funktioniert es

Jede Zuordnung leitet einen Port Ihres Rechners an eine Adresse weiter, die der SSH-Server erreichen kann:

```text
127.0.0.1:8080 (Ihr Rechner)  ──verschlüsseltes SSH──►  127.0.0.1:80 (der Server)
```

Das entspricht `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. Das Ziel wird auf dem Server aufgelöst. `127.0.0.1` bedeutet also der Server selbst, und jeder andere Host, den der Server erreichen kann, funktioniert ebenfalls.

## Entwicklung

Voraussetzungen:

- Node.js (aktuelle LTS) und npm
- Rust 1.89 oder neuer ([rustup](https://rustup.rs))
- Die Tauri-Systemabhängigkeiten für Ihr Betriebssystem, siehe [Tauri-Voraussetzungen](https://tauri.app/start/prerequisites/). Unter Debian/Ubuntu:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Danach:

```bash
npm install
npm run tauri dev    # App mit Hot Reload starten
npm run tauri build  # Installationspakete nach src-tauri/target/release/bundle/ bauen
npm run typecheck    # Typprüfung des Frontends
cargo test --manifest-path src-tauri/Cargo.toml  # Rust-Tests ausführen
```

## Projektstruktur

```text
src/         Benutzeroberfläche mit React + TypeScript
src-tauri/   Rust-Kern: SSH-Portweiterleitung, Profilspeicher, Anmeldedatenspeicher des Betriebssystems, Tray
```
