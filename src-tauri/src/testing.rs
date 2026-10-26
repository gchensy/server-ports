//! Test doubles shared by the engine and application tests: an SSH server that runs in this
//! process, a service for the tunnel to reach, and builders for profiles.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::keys::key::safe_rng;
use russh::keys::{Algorithm, PrivateKey, PublicKey};
use russh::server::{self, Msg};
use russh::{Channel, ChannelOpenFailure, Disconnect};
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use crate::model::{Auth, Forward, Profile, SecretKind};
use crate::secrets::{MemoryStore, SecretStore};

pub fn new_key() -> PrivateKey {
    PrivateKey::random(&mut safe_rng(), Algorithm::Ed25519).unwrap()
}

/// What the test server lets in.
#[derive(Clone, Default)]
pub struct Login {
    password: Option<String>,
    key: Option<PublicKey>,
}

impl Login {
    pub fn password(password: &str) -> Self {
        Self {
            password: Some(password.into()),
            key: None,
        }
    }

    pub fn key(key: &PrivateKey) -> Self {
        Self {
            password: None,
            key: Some(key.public_key().clone()),
        }
    }
}

/// One client connection of the test server. `direct-tcpip` requests are bridged to the target.
struct Peer {
    login: Login,
}

impl server::Handler for Peer {
    type Error = russh::Error;

    async fn auth_password(
        &mut self,
        _user: &str,
        password: &str,
    ) -> Result<server::Auth, Self::Error> {
        let known = self.login.password.as_deref() == Some(password);
        Ok(if known {
            server::Auth::Accept
        } else {
            server::Auth::reject()
        })
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        key: server::VerifiedPublicKey<'_>,
    ) -> Result<server::Auth, Self::Error> {
        let known = self.login.key.as_ref() == Some(key.public_key);
        Ok(if known {
            server::Auth::Accept
        } else {
            server::Auth::reject()
        })
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host: &str,
        port: u32,
        _originator: &str,
        _originator_port: u32,
        reply: server::ChannelOpenHandle,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        match TcpStream::connect((host, port as u16)).await {
            Ok(mut target) => {
                reply.accept().await;
                tokio::spawn(async move {
                    let _ = copy_bidirectional(&mut channel.into_stream(), &mut target).await;
                });
            }
            Err(_) => reply.reject(ChannelOpenFailure::ConnectFailed).await,
        }
        Ok(())
    }
}

pub struct TestServer {
    pub addr: SocketAddr,
    pub host_key: PrivateKey,
    accepting: JoinHandle<()>,
    sessions: Arc<Mutex<Vec<server::Handle>>>,
}

impl TestServer {
    pub async fn start(login: Login) -> Self {
        Self::start_at("127.0.0.1:0".parse().unwrap(), login, new_key()).await
    }

    pub async fn start_at(addr: SocketAddr, login: Login, host_key: PrivateKey) -> Self {
        let listener = TcpListener::bind(addr).await.unwrap();
        // `addr` may ask for port 0; this is the address the listener really got.
        let addr = listener.local_addr().unwrap();
        let config = Arc::new(server::Config {
            keys: vec![host_key.clone()],
            auth_rejection_time: Duration::from_millis(5),
            ..server::Config::default()
        });
        let sessions = Arc::new(Mutex::new(Vec::new()));
        let accepting = tokio::spawn({
            let sessions = sessions.clone();
            async move {
                while let Ok((socket, _)) = listener.accept().await {
                    let (config, sessions) = (config.clone(), sessions.clone());
                    let peer = Peer {
                        login: login.clone(),
                    };
                    tokio::spawn(async move {
                        if let Ok(session) = server::run_stream(config, socket, peer).await {
                            sessions.lock().unwrap().push(session.handle());
                        }
                    });
                }
            }
        });
        Self {
            addr,
            host_key,
            accepting,
            sessions,
        }
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn public_key(&self) -> PublicKey {
        self.host_key.public_key().clone()
    }

    /// Closes the listener and disconnects every client, like a server going down.
    pub async fn stop(self) {
        self.accepting.abort();
        let _ = self.accepting.await;
        let sessions = std::mem::take(&mut *self.sessions.lock().unwrap());
        for session in sessions {
            let _ = session
                .disconnect(
                    Disconnect::ByApplication,
                    "shutting down".into(),
                    "en".into(),
                )
                .await;
        }
    }
}

/// Something for the tunnel to reach: echoes everything it receives.
pub async fn echo_target() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let (mut read, mut write) = socket.split();
                let _ = tokio::io::copy(&mut read, &mut write).await;
            });
        }
    });
    port
}

pub fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

pub fn profile(server_port: u16, auth: Auth, forwards: &[(u16, u16)]) -> Profile {
    let mut profile = Profile {
        id: String::new(),
        name: "test".into(),
        host: "127.0.0.1".into(),
        port: server_port,
        username: "me".into(),
        auth,
        forwards: forwards
            .iter()
            .map(|&(local, remote)| Forward {
                id: String::new(),
                label: String::new(),
                bind_host: String::new(),
                local_port: local,
                remote_host: String::new(),
                remote_port: remote,
            })
            .collect(),
        auto_reconnect: true,
        connect_on_launch: false,
    };
    profile.normalize().unwrap();
    profile
}

pub fn secrets_with(profile: &Profile, kind: SecretKind, secret: &str) -> MemoryStore {
    let secrets = MemoryStore::default();
    secrets.store(&profile.id, kind, secret).unwrap();
    secrets
}
