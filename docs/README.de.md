<div align="center">

# Server Ports

**Ordnet Ports entfernter Server über SSH Ihrem lokalen Rechner zu.**

[English](../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Français](README.fr.md) | Deutsch | [Русский](README.ru.md)

</div>

> **Status: frühe Entwicklung.** Die erste Version ist v0.1.0. Tunnel-Engine und Oberfläche stehen, und die CI baut, testet und paketiert die App unter Linux, macOS und Windows, aber installiert und ausprobiert wurde bisher nur das `.deb`-Paket für Linux (unter Ubuntu): Die übrigen Installer sind ungetestet, und keiner von ihnen ist signiert.

Server Ports ist eine kleine plattformübergreifende Desktop-App (Windows, macOS, Linux), die Ihre SSH-Portweiterleitungen am Laufen hält. Server auswählen, benötigte Ports eintragen, und sie erscheinen auf `localhost`. Das ist dasselbe wie `ssh -L`, nur ohne Terminal-Gefummel.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="images/overview-dark.png">
    <img src="images/overview.png" alt="Das Fenster von Server Ports: links die Serverliste, rechts der ausgewählte Server mit seiner Verbindung und zwei Portweiterleitungen" width="860">
  </picture>
</p>

## Funktionen

- **Serverprofile**: ein Profil pro SSH-Server, mit beliebig vielen Port-Zuordnungen.
- **Import aus Ihrer SSH-Konfiguration**: Wählen Sie Server aus `~/.ssh/config` aus und kopieren Sie sie in einem Rutsch nach Server Ports, im Dialog „Neuer Server“ oder über den Startbildschirm. Hostname, Benutzer, Port, Schlüsseldatei (`IdentityFile`) und Weiterleitungen (`LocalForward`) werden übernommen, einschließlich der Einstellungen, die von `Host *`-Blöcken und eingebundenen `Include`-Dateien geerbt werden. Es ist eine einmalige Kopie: Ihre SSH-Konfiguration wird nur gelesen, nie verändert.
- **Drei Anmeldearten**: Passwort, Datei mit privatem Schlüssel (optional mit Passphrase) oder ssh-agent (OpenSSH-Agent oder Pageant).
- **Geheimnisse bleiben aus der Konfigurationsdatei heraus**: Auf Wunsch werden Passwörter und Schlüssel-Passphrasen im macOS-Schlüsselbund, in der Windows-Anmeldeinformationsverwaltung oder im Linux Secret Service abgelegt; andernfalls liegen sie nur im Arbeitsspeicher, solange ein Server verbunden ist.
- **Überprüfung des Host-Schlüssels**: Der Schlüssel eines bislang unbekannten Servers wird mit seinem Fingerabdruck angezeigt und nur akzeptiert, wenn Sie das bestätigen; ein geänderter Schlüssel wird gemeldet.
- **Standardmäßig privat**: Weiterleitungen lauschen auf `127.0.0.1`, sodass nur Ihr eigener Rechner sie erreichen kann.
- **Automatische Wiederverbindung**: Abgebrochene Verbindungen werden mit wachsendem Abstand (von 1 s bis 30 s) automatisch erneut versucht; ein Profil kann sich außerdem direkt beim Start der App verbinden.
- **Live-Status**: Verbindungsstatus pro Server, Listening-Status pro Zuordnung, offene Verbindungen, Datenverkehrszähler und ein kleiner Durchsatzgraph.
- **Läuft im Tray**: Beim Schließen des Fensters wird die App im Infobereich der Taskleiste ausgeblendet, statt beendet zu werden (abschaltbar); über das Tray-Menü lässt sich jeder Server ein- oder ausschalten. Die App kann auch bei der Anmeldung starten.
- **Helles und dunkles Design**, mit einer Oberfläche in neun Sprachen: Englisch, vereinfachtes und traditionelles Chinesisch, Japanisch, Koreanisch, Spanisch, Französisch, Deutsch und Russisch. Sie folgt der Systemsprache, bis Sie eine auswählen.
- **Schlank**: Mit Tauri 2 gebaut, mit Rust-Kern und der System-Webview, ohne mitgelieferten Browser.

## Installation

Laden Sie den Installer für Ihr System von der [Releases-Seite](https://github.com/gchensy/server-ports/releases) herunter und führen Sie ihn aus:

- **Windows**: der `.exe`-Installer oder die `.msi`-Datei. Die Installer sind noch nicht signiert, daher zeigt Windows SmartScreen beim ersten Start eine Warnung: Wählen Sie „Weitere Informationen“ und dann „Trotzdem ausführen“.
- **macOS**: die `.dmg`-Datei (`aarch64` für Apple Silicon, `x64` für Intel). Ziehen Sie Server Ports in „Programme“. Die App ist nur ad hoc signiert; öffnen Sie sie beim ersten Mal per Rechtsklick und „Öffnen“ oder erlauben Sie sie in den Systemeinstellungen unter „Datenschutz & Sicherheit“.
- **Linux**: das `.deb`-Paket (Debian, Ubuntu), das `.rpm`-Paket (Fedora, openSUSE) oder das `.AppImage`. Zum Beispiel:

```bash
sudo apt install ./Server*.deb     # Debian, Ubuntu
sudo dnf install ./Server*.rpm     # Fedora
chmod +x Server*.AppImage && ./Server*.AppImage
```

Unter GNOME braucht das Tray-Symbol die AppIndicator-Erweiterung (siehe „Bekannte Einschränkungen“ weiter unten). Die App aktualisiert sich nicht selbst: Zum Aktualisieren installieren Sie die neue Version über die alte.

## Erste Schritte

Die Screenshots zeigen die englische Oberfläche; die Namen der Schaltflächen im Text sind die der deutschen Oberfläche.

### 1. Server hinzufügen

Klicken Sie auf **Neuer Server** und füllen Sie das Formular aus:

- **Server**: der Host (ein Name oder eine IP-Adresse), der SSH-Port, der Benutzer und, wenn Sie möchten, ein Name für die Liste.
- **Anmeldung**: ein **Passwort**, eine **Schlüsseldatei** mit dem privaten Schlüssel (**Durchsuchen …** wählt sie aus) oder der **SSH-Agent**.
- **Portweiterleitungen**: Tragen Sie bei **Lauschen auf** den Port ein, den Sie auf Ihrem eigenen Rechner haben möchten; der Zielport auf dem Server (**Weiterleiten an**) folgt ihm, und Sie können ihn ändern. Fügen Sie für jeden benötigten Port eine Weiterleitung hinzu, auf Wunsch mit einer Bezeichnung. **Erweitert** zeigt die **Lokale Adresse**, auf der ein Port lauscht: `127.0.0.1` beschränkt ihn auf Ihren eigenen Rechner.
- **Optionen**: **Automatisch neu verbinden**, wenn die Verbindung abbricht, und **Beim Start der App verbinden**.

<p align="center">
  <img src="images/new-server.png" alt="Der Dialog „Neuer Server“, ausgefüllt" width="440">
</p>

**Speichern und verbinden** speichert den Server und verbindet sofort; **Speichern** speichert nur. Später bearbeiten und löschen Sie einen Server mit den Stift- und Papierkorb-Schaltflächen oben rechts auf seiner Seite.

Haben Sie Ihre Server schon in `~/.ssh/config`? Schalten Sie den Dialog auf **Aus SSH-Config**, haken Sie die gewünschten Server an und klicken Sie auf **Importieren**. Host, Benutzer, Port, Schlüsseldatei und die `LocalForward`-Ports werden mit übernommen; Ihre SSH-Konfiguration wird nur gelesen, nie verändert.

<p align="center">
  <img src="images/import-ssh-config.png" alt="Der Dialog „Neuer Server“ mit den in der SSH-Konfiguration gefundenen Servern" width="520">
</p>

### 2. Verbinden

Schalten Sie den Server in der Liste ein oder öffnen Sie ihn und klicken Sie auf **Verbinden**. Beim ersten Verbinden mit einem Server zeigt Server Ports den Schlüsseltyp und den Fingerabdruck an, den er vorgelegt hat, und fragt, ob Sie ihm vertrauen. Vergleichen Sie den Fingerabdruck mit dem erwarteten (auf dem Server gibt ihn `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` aus) und klicken Sie dann auf **Vertrauen und fortfahren**, oder auf **Nicht verbinden**, wenn es nicht der erwartete ist. Der Schlüssel wird danach gemerkt, und falls er sich je ändert, wird die Verbindung abgebrochen und Sie werden benachrichtigt.

<p align="center">
  <img src="images/trust-server.png" alt="Die Frage „Diesem Server vertrauen?“ mit Schlüsseltyp und Fingerabdruck" width="380">
</p>

Verlangt der Server ein Passwort oder ist Ihr Schlüssel mit einer Passphrase geschützt, werden Sie als Nächstes danach gefragt. Haken Sie **Im Schlüsselbund des Systems merken** an, um das Geheimnis im Anmeldedatenspeicher Ihres Systems abzulegen; andernfalls wird es nur im Arbeitsspeicher gehalten, solange der Server verbunden ist.

### 3. Ports nutzen

Sobald der Server verbunden ist, zeigen seine Portweiterleitungen einen grünen Punkt, und die Ports sind auf Ihrem Rechner offen: Öffnen Sie `http://127.0.0.1:8080` im Browser oder richten Sie ein beliebiges Werkzeug auf den lokalen Port, etwa einen Datenbank-Client auf `127.0.0.1:5432`.

- Die beiden Schaltflächen rechts neben einer Weiterleitung öffnen deren Adresse im Browser und kopieren sie.
- Unter der Adresse sehen Sie eine kleine Durchsatzgrafik, die gesendeten und empfangenen Daten und wie viele Verbindungen gerade offen sind und insgesamt aufgebaut wurden.
- Der Tab **Aktivität** zeigt, was auf diesem Server passiert ist (Verbinden, Wiederverbinden, Fehler): Schauen Sie zuerst dort nach, wenn etwas nicht funktioniert.
- Bricht die Verbindung ab, verbindet sich Server Ports von selbst neu (sofern Sie das für den Server nicht abgeschaltet haben) und wartet dabei jedes Mal etwas länger, bis zu 30 s. **Jetzt erneut versuchen** überspringt das Warten.

### 4. Laufen lassen

Schließen Sie das Fenster, und die Tunnel laufen im Infobereich weiter. Klicken Sie auf das Tray-Symbol oder wählen Sie in seinem Menü **Server Ports anzeigen**, um das Fenster zurückzuholen; ein zweiter Start der App bewirkt dasselbe. Das Tray-Menü schaltet außerdem jeden Server ein oder aus, verbindet oder trennt alle auf einmal und enthält **Beenden**.

Öffnen Sie **Einstellungen**, unten links im Fenster, um Sprache und Erscheinungsbild zu wählen, die App bei der Anmeldung zu starten oder sie zusammen mit ihrem Fenster zu beenden, statt sie im Infobereich zu lassen. **Server Ports beenden** schließt die App und alle ihre Tunnel.

<p align="center">
  <img src="images/settings.png" alt="Der Dialog „Einstellungen“" width="360">
</p>

## So funktioniert es

Jede Zuordnung leitet einen Port Ihres Rechners an eine Adresse weiter, die der SSH-Server erreichen kann:

```text
127.0.0.1:8080 (Ihr Rechner)  ──verschlüsseltes SSH──►  127.0.0.1:80 (der Server)
```

Das entspricht `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. Das Ziel wird auf dem Server aufgelöst. `127.0.0.1` bedeutet also der Server selbst, und jeder andere Host, den der Server erreichen kann, funktioniert ebenfalls.

## Bekannte Einschränkungen

- Noch keine tastaturinteraktive Anmeldung (Zwei-Faktor), keine Sprungserver (`ProxyJump`) und keine Host-Zertifikate.
- Nur lokale Weiterleitungen (`ssh -L`); Remote- (`-R`) und SOCKS-Weiterleitungen (`-D`) werden nicht unterstützt.
- Der Import aus `~/.ssh/config` ist eine einmalige Kopie; spätere Änderungen an der Datei werden nicht übernommen. Hosts, die `ProxyJump` oder `ProxyCommand` brauchen, werden aufgelistet, lassen sich aber noch nicht importieren; `Match`-Blöcke werden übersprungen, `RemoteForward`- und `DynamicForward`-Zeilen ignoriert. Ein Host ohne `LocalForward` öffnet das Formular, damit Sie seine Ports auswählen können.
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
docs/        Übersetzungen dieser README und die Screenshots, die sie zeigen
.github/     CI- und Release-Workflows
```
