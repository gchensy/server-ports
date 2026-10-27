<div align="center">

# Server Ports

**Associez les ports de serveurs distants à votre machine locale via SSH.**

[English](../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | Français | [Deutsch](README.de.md) | [Русский](README.ru.md)

</div>

> **Statut : développement initial.** La première version est la v0.1.0. Le moteur de tunnels et l'interface sont en place, et l'intégration continue (CI) compile, teste et empaquette l'application sous Linux, macOS et Windows, mais seul le `.deb` pour Linux a été installé et essayé jusqu'ici (sous Ubuntu) : les autres installeurs n'ont pas été testés, et aucun n'est signé.

Server Ports est une petite application de bureau multiplateforme (Windows, macOS, Linux) qui maintient vos redirections de ports SSH. Choisissez un serveur, listez les ports dont vous avez besoin, et ils apparaissent sur `localhost`. C'est la même chose que `ssh -L`, sans jongler avec le terminal.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="images/overview-dark.png">
    <img src="images/overview.png" alt="La fenêtre de Server Ports : la liste des serveurs à gauche et, à droite, le serveur sélectionné avec sa connexion et deux redirections de ports" width="860">
  </picture>
</p>

## Fonctionnalités

- **Profils de serveur** : un profil par serveur SSH, avec autant de redirections de ports que nécessaire.
- **Import depuis votre configuration SSH** : choisissez des serveurs dans `~/.ssh/config` et copiez-les d'un coup dans Server Ports, dans la fenêtre « Nouveau serveur » ou depuis l'écran d'accueil. Le nom d'hôte, l'utilisateur, le port, le fichier de clé (`IdentityFile`) et les redirections (`LocalForward`) sont repris, y compris les réglages hérités des blocs `Host *` et des fichiers `Include`. C'est une copie unique : votre configuration SSH est seulement lue, jamais modifiée.
- **Trois modes d'authentification** : mot de passe, fichier de clé privée (avec phrase secrète facultative) ou ssh-agent (agent OpenSSH ou Pageant).
- **Les secrets restent hors du fichier de configuration** : si vous le demandez, les mots de passe et les phrases secrètes des clés sont conservés dans le Trousseau d'accès de macOS, le Gestionnaire d'informations d'identification de Windows ou le Secret Service de Linux ; sinon, ils ne sont gardés en mémoire que le temps de la connexion au serveur.
- **Vérification de la clé d'hôte** : la clé d'un serveur jamais vu auparavant s'affiche avec son empreinte et n'est approuvée que si vous le confirmez ; une clé qui a changé est signalée.
- **Privé par défaut** : les redirections écoutent sur `127.0.0.1`, donc seule votre machine peut y accéder.
- **Reconnexion automatique** : les connexions interrompues sont retentées automatiquement avec un délai croissant (de 1 s à 30 s), et un profil peut se connecter dès le lancement de l'application.
- **État en direct** : état de la connexion de chaque serveur, état d'écoute de chaque redirection, connexions ouvertes, compteurs de trafic et un petit graphique de débit.
- **Résident dans la zone de notification** : fermer la fenêtre réduit l'application dans la zone de notification au lieu de la quitter (désactivable), et le menu de l'icône active ou désactive chaque serveur. L'application peut aussi démarrer à l'ouverture de session.
- **Thème clair et sombre**, avec une interface en neuf langues : anglais, chinois simplifié et traditionnel, japonais, coréen, espagnol, français, allemand et russe. Elle suit la langue du système tant que vous n'en choisissez pas une.
- **Légère** : conçue avec Tauri 2, un noyau en Rust et la vue web du système, sans navigateur embarqué.

## Installation

Téléchargez l'installeur pour votre système depuis la [page Releases](https://github.com/gchensy/server-ports/releases), puis lancez-le :

- **Windows** : l'installeur `.exe`, ou le `.msi`. Les installeurs ne sont pas encore signés, donc Windows SmartScreen affiche un avertissement au premier lancement : choisissez Informations complémentaires, puis Exécuter quand même.
- **macOS** : le `.dmg` (`aarch64` pour les Mac Apple silicon, `x64` pour les Mac Intel). Faites glisser Server Ports dans Applications. L'application n'est signée qu'« ad hoc » : au premier lancement, faites un clic droit puis Ouvrir (ou autorisez-la dans Réglages Système, Confidentialité et sécurité).
- **Linux** : le `.deb` (Debian, Ubuntu), le `.rpm` (Fedora, openSUSE) ou l'`.AppImage`. Par exemple :

```bash
sudo apt install ./Server*.deb     # Debian, Ubuntu
sudo dnf install ./Server*.rpm     # Fedora
chmod +x Server*.AppImage && ./Server*.AppImage
```

Sous GNOME, l'icône de la zone de notification nécessite l'extension AppIndicator (voir « Limitations connues » plus bas). L'application ne se met pas à jour toute seule : pour passer à une nouvelle version, installez-la par-dessus l'ancienne.

## Premiers pas

Les captures d'écran montrent l'interface en anglais ; les noms des boutons dans le texte sont ceux de l'interface en français.

### 1. Ajoutez un serveur

Cliquez sur **Nouveau serveur** et remplissez le formulaire :

- **Serveur** : l'hôte (un nom ou une adresse IP), le port SSH, l'utilisateur et, si vous le souhaitez, un nom pour la liste.
- **Connexion** : un **Mot de passe**, un **Fichier de clé** privée (**Parcourir…** permet de le choisir) ou l'**Agent SSH**.
- **Redirections de ports** : saisissez dans **Écouter sur** le port que vous voulez utiliser sur votre propre ordinateur ; le port du serveur vers lequel rediriger (**Rediriger vers**) le suit, et vous pouvez le modifier. Ajoutez une redirection pour chaque port dont vous avez besoin, avec une étiquette si vous le souhaitez. **Avancé** affiche l'**Adresse locale** sur laquelle un port écoute : `127.0.0.1` le réserve à votre ordinateur.
- **Options** : **Se reconnecter automatiquement** quand la connexion tombe, et **Se connecter au démarrage de l'application**.

<p align="center">
  <img src="images/new-server.png" alt="La boîte de dialogue « Nouveau serveur », remplie" width="440">
</p>

**Enregistrer et connecter** enregistre le serveur et se connecte aussitôt ; **Enregistrer** se contente de l'enregistrer. Ensuite, les boutons crayon et corbeille en haut à droite d'un serveur servent à le modifier et à le supprimer.

Vos serveurs sont déjà dans `~/.ssh/config` ? Basculez la boîte de dialogue sur **Depuis la config SSH**, cochez les serveurs voulus et cliquez sur **Importer**. L'hôte, l'utilisateur, le port, le fichier de clé et les ports `LocalForward` sont repris avec eux ; votre configuration SSH est seulement lue, jamais modifiée.

<p align="center">
  <img src="images/import-ssh-config.png" alt="La boîte de dialogue « Nouveau serveur » avec la liste des serveurs trouvés dans la configuration SSH" width="520">
</p>

### 2. Connectez-vous

Activez le serveur dans la liste, ou ouvrez-le et cliquez sur **Connecter**. La première fois que vous vous connectez à un serveur, Server Ports affiche le type de clé et l'empreinte qu'il a présentés, et demande si vous lui faites confiance. Comparez l'empreinte avec celle que vous attendez (sur le serveur, `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` l'affiche), puis cliquez sur **Faire confiance et continuer**, ou sur **Ne pas se connecter** si ce n'est pas la bonne. La clé est mémorisée ensuite et, si elle change un jour, la connexion est interrompue et vous en êtes informé.

<p align="center">
  <img src="images/trust-server.png" alt="La question « Faire confiance à ce serveur ? », avec le type de clé et l'empreinte" width="380">
</p>

Si le serveur demande un mot de passe, ou si votre clé est protégée par une phrase secrète, l'application vous les demande ensuite. Cochez **Mémoriser dans le trousseau du système** pour les conserver dans le magasin d'identifiants de votre système ; sinon, ils ne sont gardés en mémoire que le temps de la connexion au serveur.

### 3. Utilisez les ports

Une fois le serveur connecté, ses redirections de ports affichent un point vert et les ports sont ouverts sur votre ordinateur : ouvrez `http://127.0.0.1:8080` dans votre navigateur, ou dirigez n'importe quel outil vers le port local, par exemple un client de base de données vers `127.0.0.1:5432`.

- Les deux boutons à droite d'une redirection ouvrent son adresse dans votre navigateur et la copient.
- Sous l'adresse, vous voyez un petit graphique de débit, les données envoyées et reçues, et le nombre de connexions ouvertes à l'instant et au total.
- L'onglet **Activité** montre ce qui s'est passé sur ce serveur (connexion, reconnexion, erreurs) : regardez-y d'abord si quelque chose ne marche pas.
- Si la connexion tombe, Server Ports se reconnecte tout seul (sauf si vous l'avez désactivé pour ce serveur), en attendant un peu plus longtemps à chaque fois, jusqu'à 30 s. **Réessayer maintenant** saute l'attente.

### 4. Laissez-le tourner

Fermez la fenêtre : les tunnels continuent de fonctionner dans la zone de notification. Cliquez sur l'icône, ou choisissez **Afficher Server Ports** dans son menu, pour rouvrir la fenêtre ; lancer l'application une seconde fois fait la même chose. Le menu de l'icône permet aussi d'activer ou de désactiver chaque serveur, de tout connecter ou de tout déconnecter, et propose **Quitter**.

Ouvrez **Paramètres**, en bas à gauche de la fenêtre, pour choisir la langue et l'apparence, démarrer l'application à l'ouverture de session, ou la faire se fermer avec sa fenêtre au lieu de la laisser dans la zone de notification. **Quitter Server Ports** ferme l'application et tous ses tunnels.

<p align="center">
  <img src="images/settings.png" alt="La boîte de dialogue « Paramètres »" width="360">
</p>

## Fonctionnement

Chaque redirection fait suivre un port de votre machine vers une adresse que le serveur SSH peut atteindre :

```text
127.0.0.1:8080 (votre machine)  ──SSH chiffré──►  127.0.0.1:80 (le serveur)
```

C'est l'équivalent de `ssh -L 127.0.0.1:8080:127.0.0.1:80 user@server`. La cible est résolue côté serveur : `127.0.0.1` désigne donc le serveur lui-même, et tout autre hôte joignable depuis le serveur convient aussi.

## Limitations connues

- Pas encore d'authentification interactive au clavier (double facteur), de rebond par un autre serveur (`ProxyJump`) ni de certificats d'hôte.
- Seulement les redirections locales (`ssh -L`) ; les redirections distantes (`-R`) et SOCKS (`-D`) ne sont pas prises en charge.
- L'import depuis `~/.ssh/config` est une copie unique ; les modifications ultérieures du fichier ne sont pas reprises. Les hôtes qui exigent `ProxyJump` ou `ProxyCommand` sont listés mais ne peuvent pas encore être importés, les blocs `Match` sont ignorés, tout comme les lignes `RemoteForward` et `DynamicForward`. Un hôte sans `LocalForward` ouvre le formulaire pour que vous choisissiez ses ports.
- Sous GNOME, l'icône de la zone de notification nécessite l'extension AppIndicator. Sans elle, désactivez l'option de la zone de notification dans les réglages, ou relancez l'application pour faire revenir sa fenêtre.

## Développement

Prérequis :

- Node.js (LTS actuelle) et npm
- Rust 1.90 ou plus récent ([rustup](https://rustup.rs))
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

## Installeurs et versions

Le packaging est assuré par GitHub Actions : aucune machine macOS ou Windows n'est nécessaire de votre côté.

- **CI** (`.github/workflows/ci.yml`) s'exécute à chaque pull request et sur `main` : elle vérifie les types et compile l'interface, puis lance rustfmt, clippy et les tests Rust sous Linux, macOS et Windows.
- **Release** (`.github/workflows/release.yml`) compile les installeurs : `.deb`, `.rpm` et `.AppImage` pour Linux, `.dmg` pour macOS (Apple silicon et Intel), `.exe` et `.msi` pour Windows.

Pour publier une version, indiquez le même numéro de version dans `package.json`, `src-tauri/tauri.conf.json` et `src-tauri/Cargo.toml`, validez, puis créez le tag et poussez-le :

```bash
git tag v0.2.0
git push origin v0.2.0
```

Le workflow refuse un tag qui ne correspond pas à ces trois versions. Il joint ensuite les installeurs à une version en brouillon, que vous relisez et publiez sur GitHub. Lancer **Release** à la main (onglet Actions, Release, Run workflow) compile les mêmes installeurs sous forme d'artefacts téléchargeables, sans rien publier.

Les installeurs ne sont pas encore signés. Les versions macOS sont signées « ad hoc » : au premier lancement, il faut faire un clic droit puis Ouvrir (ou l'autoriser dans Réglages Système, Confidentialité et sécurité), et Windows SmartScreen affiche un avertissement (Informations complémentaires, Exécuter quand même). Une vraie signature et la notarisation exigent des certificats, qui doivent rester dans les secrets GitHub et jamais dans le dépôt ; voir les guides de Tauri pour [macOS](https://v2.tauri.app/distribute/sign/macos/) et [Windows](https://v2.tauri.app/distribute/sign/windows/).

## Structure du projet

```text
src/         Interface utilisateur en React + TypeScript
src-tauri/   Noyau Rust : redirection de ports SSH, stockage des profils, magasin d'identifiants du système, zone de notification
docs/        Traductions de ce README et captures d'écran qu'elles montrent
.github/     Workflows de CI et de publication
```
