//! One authenticated SSH connection and the local listeners riding on it.

use std::sync::Arc;
use std::time::Duration;

use russh::client;
use russh::{Disconnect, Preferred};
use tokio::net::{TcpStream, lookup_host};
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use super::auth::{Authenticator, KeptSecrets};
use super::env::{Env, Reporter};
use super::error::TunnelError;
use super::forward::{Forwarder, Link};
use super::handler::ClientHandler;
use crate::interaction::PROMPT_TIMEOUT;
use crate::known_hosts::key_family;
use crate::model::{Forward, Profile};

/// Per address, so that a dead IPv6 route cannot eat the whole budget before IPv4 gets a turn.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// Banner and key exchange. The time the user takes to answer the host key question is added.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);
/// A silent server is probed after this long; three unanswered probes end the session.
const KEEPALIVE: Duration = Duration::from_secs(15);
const PUBLISH_EVERY: Duration = Duration::from_millis(500);
const GOODBYE_TIMEOUT: Duration = Duration::from_secs(2);

pub struct Session {
    link: Link,
    gone: oneshot::Receiver<TunnelError>,
    forwarders: Vec<Forwarder>,
    reporter: Reporter,
    /// Stops this session's listeners and connections; a child of the tunnel's token.
    cancel: CancellationToken,
}

impl Session {
    /// Connects, checks the host key and logs in. No local port is opened yet.
    pub async fn connect(
        profile: &Profile,
        env: &Env,
        reporter: &Reporter,
        cancel: &CancellationToken,
        kept: &KeptSecrets,
    ) -> Result<Self, TunnelError> {
        let cancel = cancel.child_token();
        let result = tokio::select! {
            _ = cancel.cancelled() => Err(TunnelError::Cancelled),
            session = Self::establish(profile, env, reporter, &cancel, kept) => session,
        };
        if result.is_err() {
            cancel.cancel();
        }
        result
    }

    async fn establish(
        profile: &Profile,
        env: &Env,
        reporter: &Reporter,
        cancel: &CancellationToken,
        kept: &KeptSecrets,
    ) -> Result<Self, TunnelError> {
        reporter.info(format!("connecting to {}:{}", profile.host, profile.port));
        let socket = dial(&profile.host, profile.port).await?;
        let (handler, gone) = ClientHandler::new(profile, env, reporter, cancel);
        let handshake = client::connect_stream(Self::config(env, profile), socket, handler);
        let mut handle = timeout(HANDSHAKE_TIMEOUT + PROMPT_TIMEOUT, handshake)
            .await
            .map_err(|_| TunnelError::Timeout("during the handshake"))??;
        Authenticator::new(profile, env, reporter, cancel, kept)
            .login(&mut handle)
            .await?;
        reporter.info(format!("logged in as {}", profile.username));
        Ok(Self {
            link: Arc::new(handle),
            gone,
            forwarders: Vec::new(),
            reporter: reporter.clone(),
            cancel: cancel.clone(),
        })
    }

    fn config(env: &Env, profile: &Profile) -> Arc<client::Config> {
        // Offer the host key types we already trust first, so that a server with several host
        // keys presents the one on file instead of asking the user again about another.
        let known = env.known_hosts.known_families(&profile.host, profile.port);
        let (trusted, others): (Vec<_>, Vec<_>) = Preferred::DEFAULT
            .key
            .iter()
            .cloned()
            .partition(|algorithm| known.contains(&key_family(algorithm)));
        let mut preferred = Preferred::DEFAULT;
        preferred.key = [trusted, others].concat().into();
        Arc::new(client::Config {
            preferred,
            keepalive_interval: Some(KEEPALIVE),
            nodelay: true,
            ..client::Config::default()
        })
    }

    /// Opens the local ports. Fails only when none of them could be opened.
    pub async fn open_forwards(&mut self, forwards: &[Forward]) -> Result<(), TunnelError> {
        for forward in forwards {
            let forwarder = Forwarder::start(
                forward.clone(),
                self.link.clone(),
                self.reporter.clone(),
                self.cancel.clone(),
            )
            .await;
            self.forwarders.push(forwarder);
        }
        self.publish();
        if self.forwarders.iter().any(Forwarder::is_listening) {
            return Ok(());
        }
        let failures: Vec<_> = self
            .forwarders
            .iter()
            .filter_map(Forwarder::failure)
            .collect();
        Err(TunnelError::NoListeners(failures.join("; ")))
    }

    /// Runs until the connection is lost (`Err` says why) or the tunnel is stopped (`Ok`).
    pub async fn serve(&mut self) -> Result<(), TunnelError> {
        let mut tick = tokio::time::interval(PUBLISH_EVERY);
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => return Ok(()),
                gone = &mut self.gone => {
                    return Err(gone.unwrap_or_else(|_| TunnelError::Closed("the connection ended".into())));
                }
                _ = tick.tick() => self.publish(),
            }
        }
    }

    /// Closes the local ports (they are free again when this returns) and says goodbye to the
    /// server.
    pub async fn close(self) {
        self.cancel.cancel();
        for forwarder in self.forwarders {
            forwarder.closed().await;
        }
        let goodbye = self.link.disconnect(Disconnect::ByApplication, "", "en");
        // The peer may be gone already, so do not wait for long.
        let _ = timeout(GOODBYE_TIMEOUT, goodbye).await;
    }

    fn publish(&self) {
        self.reporter
            .set_forwards(self.forwarders.iter().map(Forwarder::runtime).collect());
    }
}

/// Tries each address of the host in turn.
async fn dial(host: &str, port: u16) -> Result<TcpStream, TunnelError> {
    let addresses = lookup_host((host, port))
        .await
        .map_err(|e| TunnelError::Network(format!("cannot resolve {host}: {e}")))?;
    let mut last = TunnelError::Network(format!("{host} has no address"));
    for address in addresses {
        match timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await {
            Ok(Ok(socket)) => return Ok(socket),
            Ok(Err(e)) => last = TunnelError::Network(format!("{address}: {e}")),
            Err(_) => last = TunnelError::Timeout("connecting"),
        }
    }
    Err(last)
}
