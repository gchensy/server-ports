//! The application without a window: profiles, settings, remembered secrets and tunnels behind
//! one object. The Tauri commands and the tray are thin wrappers around it, which keeps all of
//! this testable against a real SSH server.

use serde::Serialize;

use crate::interaction::SecretReply;
use crate::model::{
    Event, LogLine, Profile, ProfileRuntime, SecretKind, Settings, ValidationError,
};
use crate::secrets::blocking;
use crate::ssh::{Env, TunnelManager};
use crate::ssh_config::{LocalUser, SshConfig, SshHost};
use crate::store::{Store, StoreError};

/// Everything the UI needs to draw itself when it starts or after a reload. Events that race with
/// it are sorted out by `ProfileRuntime::rev`, `LogLine::seq` and the questions' ids.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub profiles: Vec<Profile>,
    pub settings: Settings,
    pub runtimes: Vec<ProfileRuntime>,
    pub logs: Vec<LogLine>,
    /// Host key and password questions that nobody has answered yet.
    pub questions: Vec<Event>,
}

/// A server found in the user's SSH config.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredHost {
    #[serde(flatten)]
    pub host: SshHost,
    /// One of the user's servers already (same host, user and port).
    pub already_added: bool,
}

/// What a command can fail with. `code` is stable, so the UI can localise it.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(
    tag = "code",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum BackendError {
    /// The profile cannot be used as typed. `field` is the code of the `ValidationError`.
    #[error("{detail}")]
    Invalid {
        field: &'static str,
        forward_index: Option<usize>,
        detail: String,
    },
    #[error("there is no such profile")]
    NotFound,
    /// The settings file cannot be written.
    #[error("{detail}")]
    Storage { detail: String },
    /// The operating system's credential store failed.
    #[error("{detail}")]
    Keychain { detail: String },
    /// The SSH config exists but cannot be read.
    #[error("{detail}")]
    SshConfig { detail: String },
}

impl From<ValidationError> for BackendError {
    fn from(error: ValidationError) -> Self {
        Self::Invalid {
            field: error.code,
            forward_index: error.forward_index,
            detail: error.message,
        }
    }
}

impl From<StoreError> for BackendError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Invalid(invalid) => invalid.into(),
            io @ StoreError::Io { .. } => Self::Storage {
                detail: io.to_string(),
            },
        }
    }
}

pub struct Backend {
    store: Store,
    manager: TunnelManager,
}

impl Backend {
    pub fn new(store: Store, env: Env) -> Self {
        Self {
            store,
            manager: TunnelManager::new(env),
        }
    }

    fn env(&self) -> &Env {
        self.manager.env()
    }

    pub fn snapshot(&self) -> Snapshot {
        let board = &self.env().board;
        Snapshot {
            profiles: self.store.profiles(),
            settings: self.store.settings(),
            runtimes: board.snapshot(),
            logs: board.all_logs(),
            questions: self.env().interaction.open_questions(),
        }
    }

    pub fn profiles(&self) -> Vec<Profile> {
        self.store.profiles()
    }

    pub fn runtime_of(&self, id: &str) -> Option<ProfileRuntime> {
        self.env().board.runtime_of(id)
    }

    // -----------------------------------------------------------------------------------------
    // Profiles
    // -----------------------------------------------------------------------------------------

    /// Validates and saves a profile (a new one when its id is empty or unknown). A tunnel that is
    /// running keeps running, unless the change affects the connection: then it is restarted.
    pub async fn save_profile(&self, profile: Profile) -> Result<Profile, BackendError> {
        let before = self.store.profile(&profile.id);
        let saved = self.store.upsert(profile)?;
        if let Some(before) = before {
            // A secret of the old login method would sit in the credential store for nothing.
            let unused = before
                .auth
                .secret_kind()
                .filter(|kind| Some(*kind) != saved.auth.secret_kind());
            if let Some(kind) = unused
                && let Err(e) = self.remove_secrets(&saved.id, vec![kind]).await
            {
                log::warn!("could not forget a saved {}: {e}", kind.as_str());
            }
            if before.needs_restart_for(&saved) {
                self.manager.restart_if_running(saved.clone()).await;
            }
        }
        Ok(saved)
    }

    pub async fn delete_profile(&self, id: &str) -> Result<(), BackendError> {
        if !self.store.delete(id)? {
            return Err(BackendError::NotFound);
        }
        self.manager.stop(id).await;
        self.env().board.forget(id);
        if let Err(e) = self.remove_secrets(id, SecretKind::ALL.to_vec()).await {
            log::warn!("could not forget the saved secrets of a deleted profile: {e}");
        }
        Ok(())
    }

    /// The servers in the user's SSH config, with the ones like a server of this app marked.
    /// Nothing is saved: the UI saves the ones the user picks through [`Backend::save_profile`],
    /// so that they are checked like any other.
    pub fn discover_ssh_hosts(
        &self,
        user: &LocalUser,
    ) -> Result<Vec<DiscoveredHost>, BackendError> {
        let config = SshConfig::load(user).map_err(|e| BackendError::SshConfig {
            detail: e.to_string(),
        })?;
        let known = self.store.profiles();
        Ok(config
            .hosts(user)
            .into_iter()
            .map(|host| DiscoveredHost {
                already_added: known.iter().any(|p| p.is_same_server(&host.profile)),
                host,
            })
            .collect())
    }

    // -----------------------------------------------------------------------------------------
    // Tunnels
    // -----------------------------------------------------------------------------------------

    /// Starts connecting. A profile that is already connecting or connected is left alone; one that
    /// has given up is tried again.
    pub async fn connect(&self, id: &str) -> Result<(), BackendError> {
        let profile = self.store.profile(id).ok_or(BackendError::NotFound)?;
        self.manager.start(profile).await;
        Ok(())
    }

    pub async fn disconnect(&self, id: &str) {
        self.manager.stop(id).await;
    }

    /// The app has just started: connects the profiles that ask for it.
    pub async fn connect_on_launch(&self) {
        for profile in self.store.profiles() {
            if profile.connect_on_launch {
                self.manager.start(profile).await;
            }
        }
    }

    pub async fn connect_all(&self) {
        for profile in self.store.profiles() {
            self.manager.start(profile).await;
        }
    }

    /// Also what the app does before it quits: no listener may outlive it.
    pub async fn disconnect_all(&self) {
        self.manager.stop_all().await;
    }

    // -----------------------------------------------------------------------------------------
    // Questions from the engine
    // -----------------------------------------------------------------------------------------

    /// `false` when the question is gone already (answered elsewhere, timed out, withdrawn).
    pub fn answer_host_key(&self, request_id: &str, trust: bool) -> bool {
        self.env().interaction.answer_host_key(request_id, trust)
    }

    /// `reply == None` cancels the question. `false` as for [`Backend::answer_host_key`].
    pub fn answer_secret(&self, request_id: &str, reply: Option<SecretReply>) -> bool {
        self.env().interaction.answer_secret(request_id, reply)
    }

    // -----------------------------------------------------------------------------------------
    // What is remembered
    // -----------------------------------------------------------------------------------------

    /// Which secrets of the profile are in the credential store.
    pub async fn saved_secrets(&self, id: &str) -> Result<Vec<SecretKind>, BackendError> {
        let id = id.to_string();
        blocking(&self.env().secrets, move |store| {
            let mut saved = Vec::new();
            for kind in SecretKind::ALL {
                if store.load(&id, kind)?.is_some() {
                    saved.push(kind);
                }
            }
            Ok(saved)
        })
        .await
        .map_err(|detail| BackendError::Keychain { detail })
    }

    pub async fn forget_secrets(&self, id: &str) -> Result<(), BackendError> {
        self.remove_secrets(id, SecretKind::ALL.to_vec())
            .await
            .map_err(|detail| BackendError::Keychain { detail })
    }

    async fn remove_secrets(&self, id: &str, kinds: Vec<SecretKind>) -> Result<(), String> {
        let id = id.to_string();
        blocking(&self.env().secrets, move |store| {
            kinds
                .into_iter()
                .try_for_each(|kind| store.remove(&id, kind))
        })
        .await
    }

    /// Stops trusting the key of the profile's server, so that the next connection asks again.
    /// For a server that was reinstalled; `false` when nothing was on file.
    pub fn forget_host_key(&self, id: &str) -> Result<bool, BackendError> {
        let profile = self.store.profile(id).ok_or(BackendError::NotFound)?;
        self.env()
            .known_hosts
            .forget(&profile.host, profile.port)
            .map_err(|e| BackendError::Storage {
                detail: e.to_string(),
            })
    }

    // -----------------------------------------------------------------------------------------
    // Settings
    // -----------------------------------------------------------------------------------------

    pub fn settings(&self) -> Settings {
        self.store.settings()
    }

    pub fn set_settings(&self, settings: Settings) -> Result<(), BackendError> {
        Ok(self.store.set_settings(settings)?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc;
    use tokio::time::{sleep, timeout};

    use super::*;
    use crate::events::ChannelSink;
    use crate::known_hosts::KnownHosts;
    use crate::model::{Auth, ConnState, ForwardState};
    use crate::secrets::MemoryStore;
    use crate::testing::{Login, TestServer, echo_target, free_port, profile};

    const WAIT: Duration = Duration::from_secs(10);

    /// A backend over a scratch directory and an in-memory credential store. The host key of
    /// `trusted` (if any) is already on file.
    struct App {
        backend: Backend,
        _dir: tempfile::TempDir,
        _events: mpsc::UnboundedReceiver<Event>,
    }

    impl App {
        fn new(trusted: Option<&TestServer>) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let known_hosts = KnownHosts::in_memory();
            if let Some(server) = trusted {
                known_hosts
                    .trust("127.0.0.1", server.port(), &server.public_key())
                    .unwrap();
            }
            let (tx, events) = mpsc::unbounded_channel();
            let env = Env::new(
                known_hosts,
                MemoryStore::default(),
                Arc::new(ChannelSink(tx)),
            );
            Self {
                backend: Backend::new(Store::load(dir.path().join("profiles.json")), env),
                _dir: dir,
                _events: events,
            }
        }

        fn runtime(&self, id: &str) -> Option<ProfileRuntime> {
            self.backend.runtime_of(id)
        }

        async fn until(&self, what: &str, mut done: impl FnMut(&Backend) -> bool) {
            let waiting = async {
                while !done(&self.backend) {
                    sleep(Duration::from_millis(10)).await;
                }
            };
            timeout(WAIT, waiting)
                .await
                .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
        }

        async fn until_state(&self, id: &str, what: &str, done: impl Fn(&ConnState) -> bool) {
            self.until(what, |b| b.runtime_of(id).is_some_and(|r| done(&r.conn)))
                .await;
        }

        /// Saves a profile with its password remembered.
        async fn save_with_password(&self, profile: Profile, password: &str) -> Profile {
            let saved = self.backend.save_profile(profile).await.unwrap();
            self.backend
                .env()
                .secrets
                .store(&saved.id, SecretKind::Password, password)
                .unwrap();
            saved
        }
    }

    async fn listens_on(port: u16) -> bool {
        TcpStream::connect(("127.0.0.1", port)).await.is_ok()
    }

    #[tokio::test]
    async fn profiles_are_validated_before_they_are_saved() {
        let app = App::new(None);
        let mut bad = profile(22, Auth::Agent, &[(free_port(), 80)]);
        bad.host = String::new();
        let error = app.backend.save_profile(bad).await.unwrap_err();
        assert!(matches!(error, BackendError::Invalid { field: "host", .. }));
        assert!(app.backend.snapshot().profiles.is_empty());

        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "invalid");
        assert_eq!(json["field"], "host");
        assert_eq!(json["forwardIndex"], serde_json::Value::Null);
        let json = serde_json::to_value(BackendError::NotFound).unwrap();
        assert_eq!(json["code"], "notFound");

        let good = profile(22, Auth::Agent, &[(free_port(), 80)]);
        let saved = app.backend.save_profile(good).await.unwrap();
        assert_eq!(app.backend.snapshot().profiles, [saved]);
    }

    #[tokio::test]
    async fn a_profile_connects_and_disconnects_on_request() {
        let target = echo_target().await;
        let server = TestServer::start(Login::password("secret")).await;
        let app = App::new(Some(&server));
        let local = free_port();
        let saved = app
            .save_with_password(
                profile(server.port(), Auth::Password, &[(local, target)]),
                "secret",
            )
            .await;

        app.backend.connect(&saved.id).await.unwrap();
        app.until_state(&saved.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;
        let runtime = app.runtime(&saved.id).unwrap();
        assert_eq!(runtime.forwards[0].state, ForwardState::Listening);
        assert!(listens_on(local).await);
        assert_eq!(app.backend.snapshot().runtimes, [runtime]);

        app.backend.disconnect(&saved.id).await;
        assert_eq!(app.runtime(&saved.id).unwrap().conn, ConnState::Idle);
        assert!(!listens_on(local).await, "the port is free again");

        let missing = app.backend.connect("nope").await.unwrap_err();
        assert!(matches!(missing, BackendError::NotFound));
    }

    #[tokio::test]
    async fn editing_a_running_profile_restarts_it_only_when_the_connection_changes() {
        let target = echo_target().await;
        let server = TestServer::start(Login::password("secret")).await;
        let app = App::new(Some(&server));
        let (first, second) = (free_port(), free_port());
        let saved = app
            .save_with_password(
                profile(server.port(), Auth::Password, &[(first, target)]),
                "secret",
            )
            .await;
        app.backend.connect(&saved.id).await.unwrap();
        app.until_state(&saved.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;
        let rev = app.runtime(&saved.id).unwrap().rev;

        let mut renamed = saved.clone();
        renamed.name = "renamed".into();
        renamed.connect_on_launch = true;
        app.backend.save_profile(renamed).await.unwrap();
        sleep(Duration::from_millis(100)).await;
        assert_eq!(
            app.runtime(&saved.id).unwrap().rev,
            rev,
            "nothing was restarted"
        );
        assert!(listens_on(first).await);

        let mut moved = saved.clone();
        moved.forwards[0].local_port = second;
        app.backend.save_profile(moved).await.unwrap();
        app.until("the new port to open", |_| {
            app.runtime(&saved.id).is_some_and(|r| {
                r.conn == ConnState::Connected && r.forwards[0].state == ForwardState::Listening
            })
        })
        .await;
        assert!(listens_on(second).await);
        assert!(!listens_on(first).await, "the old port was closed");
    }

    #[tokio::test]
    async fn editing_a_profile_that_is_not_running_does_not_start_it() {
        let server = TestServer::start(Login::password("secret")).await;
        let app = App::new(Some(&server));
        let saved = app
            .save_with_password(
                profile(server.port(), Auth::Password, &[(free_port(), 80)]),
                "secret",
            )
            .await;
        let mut edited = saved.clone();
        edited.forwards[0].remote_port = 81;
        app.backend.save_profile(edited).await.unwrap();
        sleep(Duration::from_millis(100)).await;
        assert!(app.runtime(&saved.id).is_none());
    }

    #[tokio::test]
    async fn deleting_a_profile_stops_it_and_forgets_it() {
        let target = echo_target().await;
        let server = TestServer::start(Login::password("secret")).await;
        let app = App::new(Some(&server));
        let local = free_port();
        let saved = app
            .save_with_password(
                profile(server.port(), Auth::Password, &[(local, target)]),
                "secret",
            )
            .await;
        app.backend.connect(&saved.id).await.unwrap();
        app.until_state(&saved.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;

        app.backend.delete_profile(&saved.id).await.unwrap();
        assert!(app.backend.snapshot().profiles.is_empty());
        assert!(app.runtime(&saved.id).is_none());
        assert!(!listens_on(local).await);
        assert!(
            app.backend
                .saved_secrets(&saved.id)
                .await
                .unwrap()
                .is_empty()
        );
        let again = app.backend.delete_profile(&saved.id).await.unwrap_err();
        assert!(matches!(again, BackendError::NotFound));
    }

    #[tokio::test]
    async fn changing_the_login_method_drops_the_secret_it_no_longer_uses() {
        let app = App::new(None);
        let saved = app
            .save_with_password(profile(22, Auth::Password, &[(free_port(), 80)]), "secret")
            .await;
        assert_eq!(
            app.backend.saved_secrets(&saved.id).await.unwrap(),
            [SecretKind::Password]
        );

        let mut renamed = saved.clone();
        renamed.name = "renamed".into();
        app.backend.save_profile(renamed).await.unwrap();
        assert_eq!(
            app.backend.saved_secrets(&saved.id).await.unwrap(),
            [SecretKind::Password],
            "an unrelated edit keeps the password"
        );

        let mut keyed = saved.clone();
        keyed.auth = Auth::Key {
            path: "~/.ssh/id_ed25519".into(),
        };
        app.backend.save_profile(keyed).await.unwrap();
        assert!(
            app.backend
                .saved_secrets(&saved.id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn saved_secrets_can_be_forgotten_on_request() {
        let app = App::new(None);
        let saved = app
            .save_with_password(profile(22, Auth::Password, &[(free_port(), 80)]), "secret")
            .await;
        app.backend.forget_secrets(&saved.id).await.unwrap();
        assert!(
            app.backend
                .saved_secrets(&saved.id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn an_open_question_is_in_the_snapshot_until_it_is_answered() {
        let target = echo_target().await;
        let server = TestServer::start(Login::password("secret")).await;
        let app = App::new(Some(&server));
        let saved = app
            .backend
            .save_profile(profile(
                server.port(),
                Auth::Password,
                &[(free_port(), target)],
            ))
            .await
            .unwrap();
        app.backend.connect(&saved.id).await.unwrap();

        app.until("the password question", |b| {
            !b.snapshot().questions.is_empty()
        })
        .await;
        let questions = app.backend.snapshot().questions;
        let [Event::Secret(question)] = &questions[..] else {
            panic!("expected one password question, got {questions:?}");
        };
        assert_eq!(question.kind, SecretKind::Password);
        assert_eq!(question.profile_id, saved.id);

        let reply = SecretReply {
            value: "secret".into(),
            remember: true,
        };
        assert!(app.backend.answer_secret(&question.request_id, Some(reply)));
        app.until_state(&saved.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;
        assert!(app.backend.snapshot().questions.is_empty());
        assert_eq!(
            app.backend.saved_secrets(&saved.id).await.unwrap(),
            [SecretKind::Password],
            "the password was remembered as asked"
        );
        assert!(
            !app.backend.answer_secret(&question.request_id, None),
            "answered already"
        );
    }

    #[tokio::test]
    async fn a_changed_host_key_can_be_forgotten_and_trusted_again() {
        let server = TestServer::start(Login::password("secret")).await;
        let (addr, port) = (server.addr, server.port());
        let app = App::new(Some(&server));
        let saved = app
            .save_with_password(
                profile(port, Auth::Password, &[(free_port(), 80)]),
                "secret",
            )
            .await;
        server.stop().await;
        // The same address now answers with another key, as after a reinstall.
        let _reinstalled =
            TestServer::start_at(addr, Login::password("secret"), crate::testing::new_key()).await;

        app.backend.connect(&saved.id).await.unwrap();
        app.until_state(
            &saved.id,
            "the key change to be refused",
            |c| matches!(c, ConnState::Failed { error } if error.code == "hostKeyChanged"),
        )
        .await;

        assert!(app.backend.forget_host_key(&saved.id).unwrap());
        app.backend.connect(&saved.id).await.unwrap();
        app.until("the host key question", |b| {
            !b.snapshot().questions.is_empty()
        })
        .await;
        let questions = app.backend.snapshot().questions;
        let [Event::HostKey(question)] = &questions[..] else {
            panic!("expected one host key question, got {questions:?}");
        };
        assert!(app.backend.answer_host_key(&question.request_id, true));
        app.until_state(&saved.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;
    }

    #[tokio::test]
    async fn only_profiles_that_ask_for_it_connect_on_launch() {
        let target = echo_target().await;
        let server = TestServer::start(Login::password("secret")).await;
        let app = App::new(Some(&server));
        let mut eager = profile(server.port(), Auth::Password, &[(free_port(), target)]);
        eager.connect_on_launch = true;
        let eager = app.save_with_password(eager, "secret").await;
        let lazy = app
            .save_with_password(
                profile(server.port(), Auth::Password, &[(free_port(), target)]),
                "secret",
            )
            .await;
        assert_ne!(eager.id, lazy.id);

        app.backend.connect_on_launch().await;
        app.until_state(&eager.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;
        assert!(
            app.runtime(&lazy.id).is_none(),
            "the other one was not started"
        );

        app.backend.connect_all().await;
        app.until_state(&lazy.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;
        app.backend.disconnect_all().await;
        for id in [&eager.id, &lazy.id] {
            assert_eq!(app.runtime(id).unwrap().conn, ConnState::Idle);
        }
    }

    #[tokio::test]
    async fn the_servers_of_the_ssh_config_are_offered_with_the_known_ones_marked() {
        let app = App::new(None);
        let home = tempfile::tempdir().unwrap();
        let config = home.path().join(".ssh/config");
        std::fs::create_dir_all(&config).unwrap();
        let user = LocalUser {
            name: "alice".into(),
            home: home.path().to_path_buf(),
        };

        // The file is there but cannot be read: the user hears about it.
        let error = app.backend.discover_ssh_hosts(&user).unwrap_err();
        assert!(matches!(error, BackendError::SshConfig { .. }));
        assert_eq!(serde_json::to_value(&error).unwrap()["code"], "sshConfig");

        std::fs::remove_dir(&config).unwrap();
        std::fs::write(
            &config,
            "Host web
               HostName web.example.com
               User deploy
               LocalForward 8080 localhost:80
             Host db
               HostName db.example.com
               ProxyJump bastion.example.com
            ",
        )
        .unwrap();
        let found = app.backend.discover_ssh_hosts(&user).unwrap();
        let summary = |found: &[DiscoveredHost]| -> Vec<_> {
            found
                .iter()
                .map(|h| (h.host.profile.name.clone(), h.already_added))
                .collect()
        };
        assert_eq!(
            summary(&found),
            [("web".into(), false), ("db".into(), false)]
        );

        // The same account on the same server is the same server, whatever it is called here.
        let mut known = profile(22, Auth::Agent, &[(free_port(), 80)]);
        known.name = "mine".into();
        known.host = "WEB.example.com".into();
        known.username = "deploy".into();
        app.backend.save_profile(known).await.unwrap();
        let found = app.backend.discover_ssh_hosts(&user).unwrap();
        assert_eq!(
            summary(&found),
            [("web".into(), true), ("db".into(), false)]
        );
        assert_eq!(
            app.backend.snapshot().profiles.len(),
            1,
            "nothing was saved"
        );

        let json = serde_json::to_value(&found[1]).unwrap();
        assert_eq!(json["alreadyAdded"], false);
        assert_eq!(json["proxy"], "ProxyJump");
        assert_eq!(json["profile"]["name"], "db");
        assert_eq!(json["profile"]["username"], "alice");
        assert_eq!(json["profile"]["auth"]["type"], "agent");
        assert_eq!(json["profile"]["forwards"], serde_json::json!([]));
        let json = serde_json::to_value(&found[0]).unwrap();
        assert_eq!(json["proxy"], serde_json::Value::Null);
        assert_eq!(json["profile"]["forwards"][0]["localPort"], 8080);
    }

    #[tokio::test]
    async fn settings_are_kept() {
        let app = App::new(None);
        assert!(app.backend.settings().close_to_tray);
        app.backend
            .set_settings(Settings {
                close_to_tray: false,
            })
            .unwrap();
        assert!(!app.backend.snapshot().settings.close_to_tray);
    }

    /// A port that something else holds is a problem of that mapping only.
    #[tokio::test]
    async fn a_taken_port_is_reported_without_failing_the_profile() {
        let target = echo_target().await;
        let server = TestServer::start(Login::password("secret")).await;
        let app = App::new(Some(&server));
        let held = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let taken = held.local_addr().unwrap().port();
        let saved = app
            .save_with_password(
                profile(
                    server.port(),
                    Auth::Password,
                    &[(taken, target), (free_port(), target)],
                ),
                "secret",
            )
            .await;
        app.backend.connect(&saved.id).await.unwrap();
        app.until_state(&saved.id, "the tunnel to connect", |c| {
            *c == ConnState::Connected
        })
        .await;
        let runtime = app.runtime(&saved.id).unwrap();
        assert!(matches!(
            runtime.forwards[0].state,
            ForwardState::Failed { .. }
        ));
        assert_eq!(runtime.forwards[1].state, ForwardState::Listening);
    }
}
