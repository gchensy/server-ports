<div align="center">

# Server Ports

**Associez les ports de serveurs distants à votre machine locale via SSH.**

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | Français | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **Statut : développement initial.** Il n'existe pas encore de version publiée. Le moteur de tunnels et l'interface sont en place, mais l'application n'a été compilée et lancée que sous Linux ; macOS et Windows n'ont pas été testés.

Server Ports est une petite application de bureau multiplateforme (Windows, macOS, Linux) qui maintient vos redirections de ports SSH. Choisissez un serveur, listez les ports dont vous avez besoin, et ils apparaissent sur `localhost`. C'est la même chose que `ssh -L`, sans jongler avec le terminal.

## Fonctionnalités

- **Profils de serveur** : un profil par serveur SSH, avec autant de redirections de ports que nécessaire.
- **Trois modes d'authentification** : mot de passe, fichier de clé privée (avec phrase secrète facultative) ou ssh-agent (agent OpenSSH ou Pageant).
- **Les secrets restent hors du fichier de configuration** : si vous le demandez, les mots de passe et les phrases secrètes des clés sont conservés dans le Trousseau d'accès de macOS, le Gestionnaire d'informations d'identification de Windows ou le Secret Service de Linux ; sinon, ils ne sont gardés en mémoire que le temps de la connexion au serveur.
- **Vérification de la clé d'hôte** : la clé d'un serveur jamais vu auparavant s'affiche avec son empreinte et n'est approuvée que si vous le confirmez ; une clé qui a changé est signalée.
- **Privé par défaut** : les redirections écoutent sur `127.0.0.1`, donc seule votre machine peut y accéder.
- **Reconnexion automatique** : les connexions interrompues sont retentées automatiquement avec un délai croissant (de 1 s à 30 s), et un profil peut se connecter dès le lancement de l'application.
- **État en direct** : état de la connexion de chaque serveur, état d'écoute de chaque redirection, connexions ouvertes, compteurs de trafic et un petit graphique de débit.
- **Résident dans la zone de notification** : fermer la fenêtre réduit l'application dans la zone de notification au lieu de la quitter (désactivable), et le menu de l'icône active ou désactive chaque serveur. L'application peut aussi démarrer à l'ouverture de session.
- **Thème clair et sombre**, avec une interface en anglais ou en chinois simplifié.
- **Légère** : conçue avec Tauri 2, un noyau en Rust et la vue web du système, sans navigateur embarqué.

## Fonctionnement

Chaque redirection fait suivre un port de votre machine vers une adresse que le serveur SSH peut atteindre :

```text
127.0.0.1:8080 (votre machine)  ──SSH chiffré──►  127.0.0.1:80 (le serveur)
```

C'est l'équivalent de `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. La cible est résolue côté serveur : `127.0.0.1` désigne donc le serveur lui-même, et tout autre hôte joignable depuis le serveur convient aussi.

## Limitations connues

- Pas encore d'authentification interactive au clavier (double facteur), de rebond par un autre serveur (`ProxyJump`) ni de certificats d'hôte.
- Seulement les redirections locales (`ssh -L`) ; les redirections distantes (`-R`) et SOCKS (`-D`) ne sont pas prises en charge.
- Les serveurs s'ajoutent à la main ; `~/.ssh/config` n'est pas lu.
- Sous GNOME, l'icône de la zone de notification nécessite l'extension AppIndicator. Sans elle, désactivez l'option de la zone de notification dans les réglages, ou relancez l'application pour faire revenir sa fenêtre.

## Développement

Prérequis :

- Node.js (LTS actuelle) et npm
- Rust 1.89 ou plus récent ([rustup](https://rustup.rs))
- Les dépendances système de Tauri pour votre OS ; voir les [prérequis de Tauri](https://tauri.app/start/prerequisites/). Sur Debian/Ubuntu :

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Ensuite :

```bash
npm install
npm run tauri dev    # lance l'application avec rechargement à chaud
npm run dev          # l'interface seule dans un navigateur, avec un faux backend à la place de SSH
npm run tauri build  # construit les installeurs dans src-tauri/target/release/bundle/
npm run typecheck    # vérifie les types du frontend
cargo test --manifest-path src-tauri/Cargo.toml  # lance les tests Rust
```

## Structure du projet

```text
src/         Interface utilisateur en React + TypeScript
src-tauri/   Noyau Rust : redirection de ports SSH, stockage des profils, magasin d'identifiants du système, zone de notification
```
