//! What russh calls back into during a connection: the host key check, the server's banner and
//! the news that the connection is gone.

use russh::client::{self, DisconnectReason, RemoteDisconnectInfo};
use russh::keys::{PublicKey, PublicKeyOrCertificate};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use super::env::{Env, Reporter};
use super::error::TunnelError;
use crate::known_hosts::{HostKeyCheck, fingerprint, key_family};
use crate::model::{HostKeyRequest, Profile};

pub struct ClientHandler {
    profile: Profile,
    env: Env,
    reporter: Reporter,
    /// Abandons a pending host key question when the tunnel is stopped.
    cancel: CancellationToken,
    closed: Option<oneshot::Sender<TunnelError>>,
}

impl ClientHandler {
    /// The receiver yields why the connection ended.
    pub fn new(
        profile: &Profile,
        env: &Env,
        reporter: &Reporter,
        cancel: &CancellationToken,
    ) -> (Self, oneshot::Receiver<TunnelError>) {
        let (closed, gone) = oneshot::channel();
        let handler = Self {
            profile: profile.clone(),
            env: env.clone(),
            reporter: reporter.clone(),
            cancel: cancel.clone(),
            closed: Some(closed),
        };
        (handler, gone)
    }

    /// First sight of this key: the user decides, and a "yes" is remembered.
    async fn ask_to_trust(&self, key: &PublicKey, other_known: bool) -> Result<(), TunnelError> {
        let Profile {
            id,
            name,
            host,
            port,
            ..
        } = &self.profile;
        let request = HostKeyRequest {
            request_id: String::new(),
            profile_id: id.clone(),
            profile_name: name.clone(),
            host: host.clone(),
            port: *port,
            key_type: key_family(&key.algorithm()),
            fingerprint: fingerprint(key),
            other_known,
        };
        let trusted = tokio::select! {
            trusted = self.env.interaction.confirm_host_key(request) => trusted,
            _ = self.cancel.cancelled() => return Err(TunnelError::Cancelled),
        };
        if !trusted {
            return Err(TunnelError::HostKeyRejected);
        }
        if let Err(e) = self.env.known_hosts.trust(host, *port, key) {
            self.reporter
                .warn(format!("could not save the host key: {e}"));
        }
        Ok(())
    }
}

fn describe(info: &RemoteDisconnectInfo) -> String {
    match info.message.trim() {
        "" => format!("the server closed the connection ({:?})", info.reason_code),
        message => format!("the server closed the connection: {message}"),
    }
}

impl client::Handler for ClientHandler {
    type Error = TunnelError;

    async fn check_server_key(
        &mut self,
        presented: &PublicKeyOrCertificate,
    ) -> Result<bool, TunnelError> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = presented else {
            return Err(TunnelError::HostCertificate);
        };
        match self
            .env
            .known_hosts
            .check(&self.profile.host, self.profile.port, key)
        {
            HostKeyCheck::Trusted => {}
            HostKeyCheck::Changed { known_fingerprint } => {
                return Err(TunnelError::HostKeyChanged {
                    known: known_fingerprint,
                    presented: fingerprint(key),
                });
            }
            HostKeyCheck::Unknown { other_known } => self.ask_to_trust(key, other_known).await?,
        }
        Ok(true)
    }

    async fn auth_banner(
        &mut self,
        banner: &str,
        _session: &mut client::Session,
    ) -> Result<(), TunnelError> {
        self.reporter
            .info(format!("message from the server: {}", banner.trim()));
        Ok(())
    }

    async fn disconnected(
        &mut self,
        reason: DisconnectReason<TunnelError>,
    ) -> Result<(), TunnelError> {
        let error = match reason {
            DisconnectReason::ReceivedDisconnect(info) => TunnelError::Closed(describe(&info)),
            DisconnectReason::Error(error) => error,
        };
        if let Some(closed) = self.closed.take() {
            let _ = closed.send(error);
        }
        Ok(())
    }
}
