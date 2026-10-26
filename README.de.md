<div align="center">

# Server Ports

**Ordnet Ports entfernter Server über SSH Ihrem lokalen Rechner zu.**

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | Deutsch | [Русский](README.ru.md)

</div>

> **Status: frühe Entwicklung.** Es gibt noch keine veröffentlichte Version. Tunnel-Engine und Oberfläche stehen, und die CI baut, testet und paketiert die App unter Linux, macOS und Windows, aber ausgeführt wurde sie bisher nur unter Linux; die Installer für macOS und Windows sind ungetestet.

Server Ports ist eine kleine plattformübergreifende Desktop-App (Windows, macOS, Linux), die Ihre SSH-Portweiterleitungen am Laufen hält. Server auswählen, benötigte Ports eintragen, und sie erscheinen auf `localhost`. Das ist dasselbe wie `ssh -L`, nur ohne Terminal-Gefummel.

## Funktionen

- **Serverprofile**: ein Profil pro SSH-Server, mit beliebig vielen Port-Zuordnungen.
- **Drei Anmeldearten**: Passwort, Datei mit privatem Schlüssel (optional mit Passphrase) oder ssh-agent (OpenSSH-Agent oder Pageant).
- **Geheimnisse bleiben aus der Konfigurationsdatei heraus**: Auf Wunsch werden Passwörter und Schlüssel-Passphrasen im macOS-Schlüsselbund, in der Windows-Anmeldeinformationsverwaltung oder im Linux Secret Service abgelegt; andernfalls liegen sie nur im Arbeitsspeicher, solange ein Server verbunden ist.
- **Überprüfung des Host-Schlüssels**: Der Schlüssel eines bislang unbekannten Servers wird mit seinem Fingerabdruck angezeigt und nur akzeptiert, wenn Sie das bestätigen; ein geänderter Schlüssel wird gemeldet.
- **Standardmäßig privat**: Weiterleitungen lauschen auf `127.0.0.1`, sodass nur Ihr eigener Rechner sie erreichen kann.
- **Automatische Wiederverbindung**: Abgebrochene Verbindungen werden mit wachsendem Abstand (von 1 s bis 30 s) automatisch erneut versucht; ein Profil kann sich außerdem direkt beim Start der App verbinden.
- **Live-Status**: Verbindungsstatus pro Server, Listening-Status pro Zuordnung, offene Verbindungen, Datenverkehrszähler und ein kleiner Durchsatzgraph.
- **Läuft im Tray**: Beim Schließen des Fensters wird die App im Infobereich der Taskleiste ausgeblendet, statt beendet zu werden (abschaltbar); über das Tray-Menü lässt sich jeder Server ein- oder ausschalten. Die App kann auch bei der Anmeldung starten.
- **Helles und dunkles Design**, mit einer Oberfläche in neun Sprachen: Englisch, vereinfachtes und traditionelles Chinesisch, Japanisch, Koreanisch, Spanisch, Französisch, Deutsch und Russisch. Sie folgt der Systemsprache, bis Sie eine auswählen.
- **Schlank**: Mit Tauri 2 gebaut, mit Rust-Kern und der System-Webview, ohne mitgelieferten Browser.

## So funktioniert es

Jede Zuordnung leitet einen Port Ihres Rechners an eine Adresse weiter, die der SSH-Server erreichen kann:

```text
127.0.0.1:8080 (Ihr Rechner)  ──verschlüsseltes SSH──►  127.0.0.1:80 (der Server)
```

Das entspricht `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. Das Ziel wird auf dem Server aufgelöst. `127.0.0.1` bedeutet also der Server selbst, und jeder andere Host, den der Server erreichen kann, funktioniert ebenfalls.

## Bekannte Einschränkungen

- Noch keine tastaturinteraktive Anmeldung (Zwei-Faktor), keine Sprungserver (`ProxyJump`) und keine Host-Zertifikate.
- Nur lokale Weiterleitungen (`ssh -L`); Remote- (`-R`) und SOCKS-Weiterleitungen (`-D`) werden nicht unterstützt.
- Server werden von Hand hinzugefügt; `~/.ssh/config` wird nicht gelesen.
- Unter GNOME braucht das Tray-Symbol die AppIndicator-Erweiterung. Ohne sie schalten Sie die Tray-Option in den Einstellungen aus oder starten die App erneut, um ihr Fenster zurückzuholen.

## Entwicklung

Voraussetzungen:

- Node.js (aktuelle LTS) und npm
- Rust 1.90 oder neuer ([rustup](https://rustup.rs))
- Die Tauri-Systemabhängigkeiten für Ihr Betriebssystem, siehe [Tauri-Voraussetzungen](https://tauri.app/start/prerequisites/). Unter Debian/Ubuntu:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Danach:

```bash
npm install
npm run tauri dev    # App mit Hot Reload starten
npm run dev          # nur die Oberfläche im Browser, mit einem Fake-Backend statt SSH
npm run tauri build  # Installationspakete nach src-tauri/target/release/bundle/ bauen
npm run typecheck    # Typprüfung des Frontends
cargo test --manifest-path src-tauri/Cargo.toml  # Rust-Tests ausführen
```

## Installer und Releases

Das Paketieren übernimmt GitHub Actions, Sie brauchen also keinen Rechner mit macOS oder Windows.

- **CI** (`.github/workflows/ci.yml`) läuft bei jedem Pull Request und auf `main`: Sie prüft die Typen der Oberfläche, baut sie und führt rustfmt, clippy und die Rust-Tests unter Linux, macOS und Windows aus.
- **Release** (`.github/workflows/release.yml`) baut die Installer: `.deb`, `.rpm` und `.AppImage` für Linux, `.dmg` für macOS (Apple Silicon und Intel), `.exe` und `.msi` für Windows.

Um eine Version zu veröffentlichen, tragen Sie dieselbe Versionsnummer in `package.json`, `src-tauri/tauri.conf.json` und `src-tauri/Cargo.toml` ein, committen, setzen dann den Tag und pushen ihn:

```bash
git tag v0.2.0
git push origin v0.2.0
```

Der Workflow lehnt einen Tag ab, der nicht zu diesen drei Versionen passt. Danach hängt er die Installer an ein Release im Entwurfsstatus, das Sie auf GitHub prüfen und veröffentlichen. Wenn Sie **Release** von Hand starten (Tab Actions, Release, Run workflow), werden dieselben Installer als herunterladbare Workflow-Artefakte gebaut, ohne dass etwas veröffentlicht wird.

Die Installer sind noch nicht signiert. macOS-Builds sind ad hoc signiert, daher muss man sie beim ersten Start per Rechtsklick und „Öffnen“ starten (oder in den Systemeinstellungen unter „Datenschutz & Sicherheit“ erlauben), und Windows SmartScreen zeigt eine Warnung (Weitere Informationen, Trotzdem ausführen). Echte Signierung und Notarisierung brauchen Zertifikate, die in die GitHub-Secrets gehören und niemals ins Repository; siehe die Tauri-Anleitungen für [macOS](https://v2.tauri.app/distribute/sign/macos/) und [Windows](https://v2.tauri.app/distribute/sign/windows/).

## Projektstruktur

```text
src/         Benutzeroberfläche mit React + TypeScript
src-tauri/   Rust-Kern: SSH-Portweiterleitung, Profilspeicher, Anmeldedatenspeicher des Betriebssystems, Tray
.github/     CI- und Release-Workflows
```
