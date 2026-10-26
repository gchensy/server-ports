//! Engine tests against an in-process SSH server: real sockets and real SSH. Only the user (a
//! scripted [`User`]) and the credential store (in memory) are stand-ins.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use russh::client;
use russh::keys::PrivateKey;
use russh::keys::agent::client::AgentClient;
use russh::keys::key::safe_rng;
use russh::keys::ssh_key::LineEnding;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

use super::auth::{Authenticator, KeptSecrets};
use super::handler::ClientHandler;
use super::{Env, Reporter, StatusBoard, TunnelManager};
use crate::events::{ChannelSink, NullSink};
use crate::interaction::SecretReply;
use crate::known_hosts::{HostKeyCheck, KnownHosts};
use crate::model::{
    Auth, ConnState, Event, ForwardState, HostKeyRequest, Profile, ProfileRuntime, SecretKind,
    SecretRequest,
};
use crate::secrets::{MemoryStore, SecretStore};
use crate::testing::{Login, TestServer, echo_target, free_port, new_key, profile, secrets_with};

const WAIT: Duration = Duration::from_secs(10);

fn write_key(path: &Path, key: &PrivateKey) {
    std::fs::write(path, key.to_openssh(LineEnding::LF).unwrap().as_bytes()).unwrap();
}

// ---------------------------------------------------------------------------------------------
// The client side: the engine, and a scripted user to answer its questions
// ---------------------------------------------------------------------------------------------

/// What the person in front of the app does when the engine asks something.
#[derive(Default)]
struct User {
    trusts_host_keys: bool,
    /// What gets typed into a secret prompt; `None` presses "cancel".
    types: Option<String>,
    remembers: bool,
    /// Leaves every question unanswered.
    silent: bool,
    host_key_questions: Vec<HostKeyRequest>,
    secret_questions: Vec<SecretRequest>,
}

struct Harness {
    manager: TunnelManager,
    events: mpsc::UnboundedReceiver<Event>,
    user: User,
}

impl Harness {
    fn new(known_hosts: KnownHosts, secrets: MemoryStore) -> Self {
        let (tx, events) = mpsc::unbounded_channel();
        let env = Env::new(known_hosts, secrets, Arc::new(ChannelSink(tx)));
        Self {
            manager: TunnelManager::new(env),
            events,
            user: User {
                trusts_host_keys: true,
                ..User::default()
            },
        }
    }

    fn env(&self) -> &Env {
        self.manager.env()
    }

    /// The next event, after the user has reacted to it.
    async fn next_event(&mut self) -> Event {
        let event = self.events.recv().await.expect("the event channel closed");
        match &event {
            Event::HostKey(request) => {
                self.user.host_key_questions.push(request.clone());
                if !self.user.silent {
                    let hub = &self.manager.env().interaction;
                    hub.answer_host_key(&request.request_id, self.user.trusts_host_keys);
                }
            }
            Event::Secret(request) => {
                self.user.secret_questions.push(request.clone());
                if !self.user.silent {
                    let reply = self.user.types.clone().map(|value| SecretReply {
                        value,
                        remember: self.user.remembers,
                    });
                    self.manager
                        .env()
                        .interaction
                        .answer_secret(&request.request_id, reply);
                }
            }
            _ => {}
        }
        event
    }

    /// Waits for a state update that satisfies `done`.
    async fn wait_for(
        &mut self,
        what: &str,
        done: impl Fn(&ProfileRuntime) -> bool,
    ) -> ProfileRuntime {
        let waiting = async {
            loop {
                if let Event::State(state) = self.next_event().await
                    && done(&state)
                {
                    return state;
                }
            }
        };
        timeout(WAIT, waiting)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }

    /// Waits until the board shows what `done` looks for.
    async fn wait_until(&mut self, what: &str, done: impl Fn(&StatusBoard) -> bool) {
        let board = self.manager.env().board.clone();
        let waiting = async {
            while !done(&board) {
                self.next_event().await;
            }
        };
        timeout(WAIT, waiting)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
    }

    /// The tunnel's task has ended (it gave up).
    async fn until_stopped(&self, id: &str) {
        for _ in 0..100 {
            if !self.manager.is_running(id).await {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("the tunnel is still running");
    }
}

fn connected(state: &ProfileRuntime) -> bool {
    state.conn == ConnState::Connected
}

fn idle(state: &ProfileRuntime) -> bool {
    state.conn == ConnState::Idle
}

fn failed(state: &ProfileRuntime) -> bool {
    matches!(state.conn, ConnState::Failed { .. })
}

fn reconnecting(state: &ProfileRuntime) -> bool {
    matches!(state.conn, ConnState::Reconnecting { .. })
}

fn error_code(state: &ProfileRuntime) -> &str {
    match &state.conn {
        ConnState::Failed { error } | ConnState::Reconnecting { error, .. } => &error.code,
        other => panic!("{other:?} carries no error"),
    }
}

/// Sends `payload` through `stream` and checks that the echo service sends the same back.
async fn roundtrip(stream: &mut TcpStream, payload: &[u8]) {
    let (mut read, mut write) = stream.split();
    let writing = async { write.write_all(payload).await.unwrap() };
    let reading = async {
        let mut echoed = vec![0u8; payload.len()];
        read.read_exact(&mut echoed).await.unwrap();
        assert!(echoed == payload, "the echo differs from what was sent");
    };
    timeout(WAIT, async { tokio::join!(writing, reading) })
        .await
        .expect("the echo did not complete");
}

// ---------------------------------------------------------------------------------------------
// Forwarding
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn forwards_a_local_port_through_the_server() {
    let target = echo_target().await;
    let server = TestServer::start(Login::password("secret")).await;
    let local = free_port();
    let profile = profile(server.port(), Auth::Password, &[(local, target)]);
    let secrets = secrets_with(&profile, SecretKind::Password, "secret");
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to connect", connected).await;
    assert!(state.connected_at.is_some());
    assert_eq!(
        h.user.host_key_questions.len(),
        1,
        "first contact asks about the host key"
    );
    assert_eq!(h.user.host_key_questions[0].key_type, "ssh-ed25519");
    assert!(
        h.user.secret_questions.is_empty(),
        "the saved password was used"
    );
    let check = h
        .env()
        .known_hosts
        .check("127.0.0.1", server.port(), &server.public_key());
    assert_eq!(check, HostKeyCheck::Trusted, "the answer is remembered");

    let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let mut client = TcpStream::connect(("127.0.0.1", local)).await.unwrap();
    roundtrip(&mut client, &payload).await;
    let len = payload.len() as u64;
    let busy = h
        .wait_for("the traffic to be counted", |s| {
            s.forwards.first().is_some_and(|f| {
                f.state == ForwardState::Listening
                    && f.active == 1
                    && f.bytes_up >= len
                    && f.bytes_down >= len
            })
        })
        .await;
    assert_eq!(busy.forwards[0].total, 1);
    drop(client);
    h.wait_for("the connection to be released", |s| {
        s.forwards
            .first()
            .is_some_and(|f| f.active == 0 && f.total == 1)
    })
    .await;

    h.manager.stop(&profile.id).await;
    h.wait_for("the tunnel to go idle", idle).await;
    assert!(
        TcpStream::connect(("127.0.0.1", local)).await.is_err(),
        "the local port is closed again"
    );
}

#[tokio::test]
async fn a_port_that_is_taken_only_fails_its_own_mapping() {
    let target = echo_target().await;
    let server = TestServer::start(Login::password("secret")).await;
    let held = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let taken = held.local_addr().unwrap().port();
    let free = free_port();
    let profile = profile(
        server.port(),
        Auth::Password,
        &[(taken, target), (free, target)],
    );
    let secrets = secrets_with(&profile, SecretKind::Password, "secret");
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);

    h.manager.start(profile).await;
    let state = h.wait_for("the tunnel to connect", connected).await;
    match &state.forwards[0].state {
        ForwardState::Failed { error } => assert_eq!(error.code, "addrInUse"),
        other => panic!("expected the taken port to fail, got {other:?}"),
    }
    assert_eq!(state.forwards[1].state, ForwardState::Listening);
    let mut client = TcpStream::connect(("127.0.0.1", free)).await.unwrap();
    roundtrip(&mut client, b"the other mapping works").await;
}

#[tokio::test]
async fn without_any_free_port_the_tunnel_fails_for_good() {
    let target = echo_target().await;
    let server = TestServer::start(Login::password("secret")).await;
    let held = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let taken = held.local_addr().unwrap().port();
    let profile = profile(server.port(), Auth::Password, &[(taken, target)]);
    let secrets = secrets_with(&profile, SecretKind::Password, "secret");
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "noListeners");
    h.until_stopped(&profile.id).await;
}

#[tokio::test]
async fn a_target_the_server_cannot_reach_closes_just_that_connection() {
    let server = TestServer::start(Login::password("secret")).await;
    let local = free_port();
    let nothing_listens_here = free_port();
    let profile = profile(
        server.port(),
        Auth::Password,
        &[(local, nothing_listens_here)],
    );
    let secrets = secrets_with(&profile, SecretKind::Password, "secret");
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);

    h.manager.start(profile.clone()).await;
    h.wait_for("the tunnel to connect", connected).await;
    let mut client = TcpStream::connect(("127.0.0.1", local)).await.unwrap();
    let mut buf = [0u8; 1];
    let read = timeout(WAIT, client.read(&mut buf))
        .await
        .expect("the connection stayed open");
    assert!(matches!(read, Ok(0) | Err(_)), "the connection is closed");

    let logs = h.env().board.logs(&profile.id);
    assert!(
        logs.iter()
            .any(|line| line.message.contains("could not connect")),
        "the refusal is in the log: {logs:?}"
    );
    assert!(
        connected(&h.env().board.runtime_of(&profile.id).unwrap()),
        "the session carries on"
    );
}

#[tokio::test]
async fn stop_all_disconnects_every_tunnel() {
    let target = echo_target().await;
    let server = TestServer::start(Login::password("secret")).await;
    let secrets = MemoryStore::default();
    let profiles: Vec<Profile> = (0..2)
        .map(|_| {
            let profile = profile(server.port(), Auth::Password, &[(free_port(), target)]);
            secrets
                .store(&profile.id, SecretKind::Password, "secret")
                .unwrap();
            profile
        })
        .collect();
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);

    for profile in &profiles {
        h.manager.start(profile.clone()).await;
    }
    let ids: Vec<_> = profiles.iter().map(|p| p.id.clone()).collect();
    let all = |want: ConnState| {
        let ids = ids.clone();
        move |board: &StatusBoard| {
            ids.iter()
                .all(|id| board.runtime_of(id).is_some_and(|r| r.conn == want))
        }
    };
    h.wait_until("both tunnels to connect", all(ConnState::Connected))
        .await;
    h.manager.stop_all().await;
    h.wait_until("both tunnels to go idle", all(ConnState::Idle))
        .await;
    assert!(!h.manager.is_running(&ids[0]).await);
}

// ---------------------------------------------------------------------------------------------
// Connection loss
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn reconnects_when_the_server_comes_back() {
    let target = echo_target().await;
    let login = Login::password("secret");
    let server = TestServer::start(login.clone()).await;
    let (addr, host_key) = (server.addr, server.host_key.clone());
    let local = free_port();
    let profile = profile(addr.port(), Auth::Password, &[(local, target)]);
    let secrets = secrets_with(&profile, SecretKind::Password, "secret");
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);

    h.manager.start(profile).await;
    h.wait_for("the first connection", connected).await;

    server.stop().await;
    let lost = h.wait_for("the loss to be noticed", reconnecting).await;
    assert_eq!(error_code(&lost), "closed");
    assert!(matches!(
        lost.conn,
        ConnState::Reconnecting { attempt: 1, .. }
    ));

    let _back = TestServer::start_at(addr, login, host_key).await;
    h.wait_for("the second connection", connected).await;
    assert_eq!(
        h.user.host_key_questions.len(),
        1,
        "the key was trusted the first time"
    );
    let mut client = TcpStream::connect(("127.0.0.1", local)).await.unwrap();
    roundtrip(&mut client, b"after the restart").await;
}

#[tokio::test]
async fn an_unreachable_server_is_retried_later() {
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    let profile = profile(free_port(), Auth::Agent, &[(free_port(), 1)]);

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("a retry to be scheduled", reconnecting).await;
    assert_eq!(error_code(&state), "network");
    assert!(matches!(
        state.conn,
        ConnState::Reconnecting { attempt: 1, .. }
    ));

    h.manager.stop(&profile.id).await;
    h.wait_for("the tunnel to go idle", idle).await;
}

#[tokio::test]
async fn without_auto_reconnect_a_lost_server_is_a_failure() {
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    let mut profile = profile(free_port(), Auth::Agent, &[(free_port(), 1)]);
    profile.auto_reconnect = false;

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "network");
    h.until_stopped(&profile.id).await;
}

// ---------------------------------------------------------------------------------------------
// Host keys
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_changed_host_key_is_never_trusted_silently() {
    let server = TestServer::start(Login::password("secret")).await;
    let known = KnownHosts::in_memory();
    known
        .trust("127.0.0.1", server.port(), new_key().public_key())
        .unwrap();
    let profile = profile(server.port(), Auth::Password, &[(free_port(), 1)]);
    let secrets = secrets_with(&profile, SecretKind::Password, "secret");
    let mut h = Harness::new(known, secrets);

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "hostKeyChanged");
    assert!(
        h.user.host_key_questions.is_empty(),
        "nobody is asked to wave it through"
    );
    h.until_stopped(&profile.id).await;
}

#[tokio::test]
async fn a_refused_host_key_ends_the_attempt_and_is_not_saved() {
    let server = TestServer::start(Login::password("secret")).await;
    let profile = profile(server.port(), Auth::Password, &[(free_port(), 1)]);
    let secrets = secrets_with(&profile, SecretKind::Password, "secret");
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);
    h.user.trusts_host_keys = false;

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "hostKeyRejected");
    let check = h
        .env()
        .known_hosts
        .check("127.0.0.1", server.port(), &server.public_key());
    assert_eq!(check, HostKeyCheck::Unknown { other_known: false });
    h.until_stopped(&profile.id).await;
}

#[tokio::test]
async fn stopping_while_the_host_key_question_is_open_withdraws_it() {
    let server = TestServer::start(Login::password("secret")).await;
    let profile = profile(server.port(), Auth::Password, &[(free_port(), 1)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    h.user.silent = true;

    h.manager.start(profile.clone()).await;
    let request = timeout(WAIT, async {
        loop {
            if let Event::HostKey(request) = h.next_event().await {
                return request;
            }
        }
    })
    .await
    .expect("no host key question was asked");

    h.manager.stop(&profile.id).await;
    let (mut idle_seen, mut withdrawn) = (false, false);
    timeout(WAIT, async {
        while !(idle_seen && withdrawn) {
            match h.next_event().await {
                Event::State(state) if idle(&state) => idle_seen = true,
                Event::PromptClosed { request_id } if request_id == request.request_id => {
                    withdrawn = true;
                }
                _ => {}
            }
        }
    })
    .await
    .expect("the tunnel did not go idle and withdraw the question");
    let late_answer = h
        .env()
        .interaction
        .answer_host_key(&request.request_id, true);
    assert!(!late_answer, "the question no longer exists");
}

// ---------------------------------------------------------------------------------------------
// Logging in
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_rejected_saved_password_is_replaced_by_the_one_typed() {
    let target = echo_target().await;
    let server = TestServer::start(Login::password("secret")).await;
    let profile = profile(server.port(), Auth::Password, &[(free_port(), target)]);
    let secrets = secrets_with(&profile, SecretKind::Password, "stale");
    let mut h = Harness::new(KnownHosts::in_memory(), secrets);
    h.user.types = Some("secret".into());
    h.user.remembers = true;

    h.manager.start(profile.clone()).await;
    h.wait_for("the tunnel to connect", connected).await;
    assert_eq!(h.user.secret_questions.len(), 1);
    assert_eq!(h.user.secret_questions[0].kind, SecretKind::Password);
    assert!(
        h.user.secret_questions[0].retry,
        "the user is told the saved one was rejected"
    );
    let saved = h
        .env()
        .secrets
        .load(&profile.id, SecretKind::Password)
        .unwrap();
    assert_eq!(saved.as_deref(), Some("secret"));
}

#[tokio::test]
async fn a_password_is_only_remembered_when_asked() {
    let target = echo_target().await;
    let server = TestServer::start(Login::password("secret")).await;
    let profile = profile(server.port(), Auth::Password, &[(free_port(), target)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    h.user.types = Some("secret".into());

    h.manager.start(profile.clone()).await;
    h.wait_for("the tunnel to connect", connected).await;
    assert!(!h.user.secret_questions[0].retry);
    let saved = h
        .env()
        .secrets
        .load(&profile.id, SecretKind::Password)
        .unwrap();
    assert_eq!(saved, None);
}

#[tokio::test]
async fn a_typed_password_is_kept_for_the_reconnects_of_the_running_tunnel() {
    let target = echo_target().await;
    let login = Login::password("secret");
    let server = TestServer::start(login.clone()).await;
    let (addr, host_key) = (server.addr, server.host_key.clone());
    let profile = profile(addr.port(), Auth::Password, &[(free_port(), target)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    h.user.types = Some("secret".into());

    h.manager.start(profile.clone()).await;
    h.wait_for("the first connection", connected).await;
    server.stop().await;
    h.wait_for("the loss to be noticed", reconnecting).await;
    let _back = TestServer::start_at(addr, login, host_key).await;
    h.wait_for("the second connection", connected).await;
    assert_eq!(
        h.user.secret_questions.len(),
        1,
        "typed once, not per login"
    );
    let saved = h
        .env()
        .secrets
        .load(&profile.id, SecretKind::Password)
        .unwrap();
    assert_eq!(saved, None, "kept in memory only");

    // Switching the profile off forgets it again.
    h.manager.stop(&profile.id).await;
    h.wait_for("the tunnel to go idle", idle).await;
    h.manager.start(profile).await;
    h.wait_for("the third connection", connected).await;
    assert_eq!(h.user.secret_questions.len(), 2);
}

#[tokio::test]
async fn a_kept_password_the_server_turns_down_is_asked_for_again() {
    let target = echo_target().await;
    let server = TestServer::start(Login::password("secret")).await;
    let profile = profile(server.port(), Auth::Password, &[(free_port(), target)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    h.user.types = Some("secret".into());
    h.manager.start(profile.clone()).await;
    h.wait_for("the first connection", connected).await;

    // The server's password changes while the tunnel is down.
    let (addr, host_key) = (server.addr, server.host_key.clone());
    server.stop().await;
    h.wait_for("the loss to be noticed", reconnecting).await;
    let _back = TestServer::start_at(addr, Login::password("changed"), host_key).await;
    h.user.types = Some("changed".into());
    h.wait_for("the second connection", connected).await;

    let retries: Vec<_> = h.user.secret_questions.iter().map(|q| q.retry).collect();
    assert_eq!(
        retries,
        [false, true],
        "the kept password failed, then the user is asked"
    );
}

#[tokio::test]
async fn cancelling_the_password_prompt_fails_without_retrying() {
    let server = TestServer::start(Login::password("secret")).await;
    let profile = profile(server.port(), Auth::Password, &[(free_port(), 1)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    h.user.types = None;

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "cancelled");
    assert_eq!(h.user.secret_questions.len(), 1);
    h.until_stopped(&profile.id).await;
}

#[tokio::test]
async fn a_wrong_password_is_asked_for_a_few_times_then_fails() {
    let server = TestServer::start(Login::password("secret")).await;
    let profile = profile(server.port(), Auth::Password, &[(free_port(), 1)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    h.user.types = Some("nope".into());

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "auth");
    let retries: Vec<_> = h.user.secret_questions.iter().map(|q| q.retry).collect();
    assert_eq!(retries, [false, true, true]);
    h.until_stopped(&profile.id).await;
}

#[tokio::test]
async fn logs_in_with_a_key_file() {
    let target = echo_target().await;
    let key = new_key();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("id_ed25519");
    write_key(&path, &key);
    let server = TestServer::start(Login::key(&key)).await;
    let auth = Auth::Key {
        path: path.display().to_string(),
    };
    let profile = profile(server.port(), auth, &[(free_port(), target)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());

    h.manager.start(profile).await;
    h.wait_for("the tunnel to connect", connected).await;
    assert!(
        h.user.secret_questions.is_empty(),
        "an unencrypted key needs no question"
    );
}

#[tokio::test]
async fn an_encrypted_key_file_asks_for_its_passphrase() {
    let target = echo_target().await;
    let key = new_key();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("id_ed25519");
    write_key(&path, &key.encrypt(&mut safe_rng(), "hunter2").unwrap());
    let server = TestServer::start(Login::key(&key)).await;
    let auth = Auth::Key {
        path: path.display().to_string(),
    };
    let profile = profile(server.port(), auth, &[(free_port(), target)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());
    h.user.types = Some("hunter2".into());
    h.user.remembers = true;

    h.manager.start(profile.clone()).await;
    h.wait_for("the tunnel to connect", connected).await;
    assert_eq!(h.user.secret_questions.len(), 1);
    assert_eq!(h.user.secret_questions[0].kind, SecretKind::Passphrase);
    let saved = h
        .env()
        .secrets
        .load(&profile.id, SecretKind::Passphrase)
        .unwrap();
    assert_eq!(saved.as_deref(), Some("hunter2"));
}

#[tokio::test]
async fn a_key_the_server_does_not_know_is_refused() {
    let known_to_server = new_key();
    let ours = new_key();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("id_ed25519");
    write_key(&path, &ours);
    let server = TestServer::start(Login::key(&known_to_server)).await;
    let auth = Auth::Key {
        path: path.display().to_string(),
    };
    let profile = profile(server.port(), auth, &[(free_port(), 1)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "auth");
    h.until_stopped(&profile.id).await;
}

#[tokio::test]
async fn a_missing_key_file_is_reported_as_such() {
    let server = TestServer::start(Login::password("secret")).await;
    let auth = Auth::Key {
        path: "/nonexistent/id_ed25519".into(),
    };
    let profile = profile(server.port(), auth, &[(free_port(), 1)]);
    let mut h = Harness::new(KnownHosts::in_memory(), MemoryStore::default());

    h.manager.start(profile.clone()).await;
    let state = h.wait_for("the tunnel to give up", failed).await;
    assert_eq!(error_code(&state), "keyFile");
    h.until_stopped(&profile.id).await;
}

/// Logs in over a fresh connection to `server` with the agent behind `agent_end`, the way the
/// engine does for `Auth::Agent`.
async fn agent_login(
    server: &TestServer,
    agent: &mut AgentClient<tokio::io::DuplexStream>,
) -> Result<(), super::TunnelError> {
    let profile = profile(server.port(), Auth::Agent, &[(free_port(), 1)]);
    let known = KnownHosts::in_memory();
    known
        .trust("127.0.0.1", server.port(), &server.public_key())
        .unwrap();
    let env = Env::new(known, MemoryStore::default(), Arc::new(NullSink));
    let reporter = Reporter::new(env.board.clone(), &profile.id);
    let cancel = CancellationToken::new();
    let (handler, _gone) = ClientHandler::new(&profile, &env, &reporter, &cancel);
    let socket = TcpStream::connect(server.addr).await.unwrap();
    let mut handle = client::connect_stream(Arc::new(client::Config::default()), socket, handler)
        .await
        .unwrap();
    Authenticator::new(&profile, &env, &reporter, &cancel, &KeptSecrets::default())
        .agent(&mut handle, agent)
        .await
}

/// An ssh-agent, in this process, holding `keys`.
async fn agent_holding(keys: &[&PrivateKey]) -> AgentClient<tokio::io::DuplexStream> {
    let (client_end, agent_end) = tokio::io::duplex(64 * 1024);
    let connections = futures::stream::once(futures::future::ready(Ok(agent_end)));
    tokio::spawn(russh::keys::agent::server::serve(connections, ()));
    let mut agent = AgentClient::connect(client_end);
    for key in keys {
        agent.add_identity(key, &[]).await.unwrap();
    }
    agent
}

#[tokio::test]
async fn logs_in_with_a_key_held_by_ssh_agent() {
    let key = new_key();
    let server = TestServer::start(Login::key(&key)).await;
    let mut agent = agent_holding(&[&new_key(), &key]).await;

    let login = agent_login(&server, &mut agent).await;
    assert!(login.is_ok(), "{login:?}");
}

#[tokio::test]
async fn an_agent_without_an_accepted_key_is_refused() {
    let server = TestServer::start(Login::key(&new_key())).await;
    let mut agent = agent_holding(&[&new_key()]).await;

    let login = agent_login(&server, &mut agent).await;
    assert_eq!(login.unwrap_err().code(), "auth");
}

#[tokio::test]
async fn an_empty_agent_is_reported_as_such() {
    let server = TestServer::start(Login::key(&new_key())).await;
    let mut agent = agent_holding(&[]).await;

    let login = agent_login(&server, &mut agent).await;
    assert_eq!(login.unwrap_err().code(), "agent");
}
