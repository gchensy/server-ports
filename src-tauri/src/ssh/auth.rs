//! Logging in over a fresh connection with a password, a private key file or ssh-agent, asking
//! the user (and the OS credential store) for whatever is missing.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use russh::MethodKind;
use russh::client::{AuthResult, Handle};
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::{self, HashAlg, PrivateKey, PrivateKeyWithHashAlg};
use tokio_util::sync::CancellationToken;

use super::env::{Env, Reporter};
use super::error::TunnelError;
use super::handler::ClientHandler;
use crate::interaction::SecretReply;
use crate::model::{Auth, Profile, SecretKind, SecretRequest};
use crate::secrets::blocking;
use crate::util::expand_tilde;

/// How many secrets are tried before giving up; servers hang up after about six failed attempts.
const MAX_ATTEMPTS: usize = 3;

type Agent = AgentClient<Box<dyn AgentStream + Send + Unpin>>;

struct Secret {
    value: String,
    /// The user asked to keep it in the OS credential store.
    remember: bool,
}

/// What the user typed and the server accepted, kept in memory for as long as the tunnel runs, so
/// that logging in again after a lost connection does not ask again.
#[derive(Default)]
pub struct KeptSecrets(Mutex<HashMap<SecretKind, String>>);

impl KeptSecrets {
    /// Removed on the way out: a secret the server turns down is not offered a second time.
    fn take(&self, kind: SecretKind) -> Option<String> {
        self.0.lock().unwrap().remove(&kind)
    }

    fn keep(&self, kind: SecretKind, value: String) {
        self.0.lock().unwrap().insert(kind, value);
    }
}

/// The secrets to try, one by one: the one kept from the last login or held by the credential
/// store first, then whatever the user types.
struct Attempts<'a> {
    auth: &'a Authenticator<'a>,
    kind: SecretKind,
    stored: Option<String>,
    tried: usize,
}

impl Attempts<'_> {
    /// `None` once the attempts are used up.
    async fn next_secret(&mut self) -> Result<Option<Secret>, TunnelError> {
        if self.tried == MAX_ATTEMPTS {
            return Ok(None);
        }
        let rejected = self.tried > 0;
        self.tried += 1;
        if let Some(value) = self.stored.take() {
            return Ok(Some(Secret {
                value,
                remember: false,
            }));
        }
        let SecretReply { value, remember } = self.auth.ask(self.kind, rejected).await?;
        Ok(Some(Secret { value, remember }))
    }
}

pub struct Authenticator<'a> {
    profile: &'a Profile,
    env: &'a Env,
    reporter: &'a Reporter,
    /// Abandons a pending question when the tunnel is stopped.
    cancel: &'a CancellationToken,
    kept: &'a KeptSecrets,
}

impl<'a> Authenticator<'a> {
    pub fn new(
        profile: &'a Profile,
        env: &'a Env,
        reporter: &'a Reporter,
        cancel: &'a CancellationToken,
        kept: &'a KeptSecrets,
    ) -> Self {
        Self {
            profile,
            env,
            reporter,
            cancel,
            kept,
        }
    }

    pub async fn login(&self, handle: &mut Handle<ClientHandler>) -> Result<(), TunnelError> {
        match &self.profile.auth {
            Auth::Password => self.password(handle).await,
            Auth::Key { path } => self.key(handle, path).await,
            Auth::Agent => {
                let mut agent = connect_agent()
                    .await
                    .map_err(|e| TunnelError::Agent(format!("cannot reach ssh-agent: {e}")))?;
                self.agent(handle, &mut agent).await
            }
        }
    }

    async fn password(&self, handle: &mut Handle<ClientHandler>) -> Result<(), TunnelError> {
        // Find out what the server takes before asking the user for a password it never will.
        // (Servers differ in what they list after a failed attempt, so only this answer counts.)
        match handle.authenticate_none(&self.profile.username).await? {
            AuthResult::Success => return Ok(()),
            AuthResult::Failure {
                remaining_methods, ..
            } if !remaining_methods.contains(&MethodKind::Password) => {
                return Err(TunnelError::Auth(format!(
                    "the server does not accept passwords (it accepts: {})",
                    method_names(&remaining_methods)
                )));
            }
            AuthResult::Failure { .. } => {}
        }
        let mut passwords = self.attempts(SecretKind::Password).await;
        while let Some(password) = passwords.next_secret().await? {
            let result = handle
                .authenticate_password(&self.profile.username, password.value.clone())
                .await?;
            if result.success() {
                self.accepted(SecretKind::Password, password).await;
                return Ok(());
            }
        }
        Err(TunnelError::Auth("the server rejected the password".into()))
    }

    async fn key(&self, handle: &mut Handle<ClientHandler>, path: &str) -> Result<(), TunnelError> {
        let key = self.unlock_key(&expand_tilde(path)).await?;
        let hash = signing_hash(handle, key.algorithm().is_rsa()).await?;
        let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
        match handle
            .authenticate_publickey(&self.profile.username, key)
            .await?
        {
            AuthResult::Success => Ok(()),
            AuthResult::Failure { .. } => Err(TunnelError::Auth(
                "the server did not accept the key".into(),
            )),
        }
    }

    /// Reads the key file; an encrypted one is opened with the stored passphrase or one the
    /// user types.
    async fn unlock_key(&self, path: &Path) -> Result<PrivateKey, TunnelError> {
        match read_key(path, None).await {
            Err(keys::Error::KeyIsEncrypted) => {}
            other => return other.map_err(|e| key_file_error(path, &e)),
        }
        let mut passphrases = self.attempts(SecretKind::Passphrase).await;
        while let Some(passphrase) = passphrases.next_secret().await? {
            if let Ok(key) = read_key(path, Some(passphrase.value.clone())).await {
                self.accepted(SecretKind::Passphrase, passphrase).await;
                return Ok(key);
            }
        }
        Err(TunnelError::KeyFile(format!(
            "{}: the passphrase does not open the key",
            path.display()
        )))
    }

    /// Offers the agent's keys to the server one after the other.
    pub(super) async fn agent<S: AgentStream + Unpin + Send>(
        &self,
        handle: &mut Handle<ClientHandler>,
        agent: &mut AgentClient<S>,
    ) -> Result<(), TunnelError> {
        let identities = agent
            .request_identities()
            .await
            .map_err(|e| TunnelError::Agent(format!("ssh-agent: {e}")))?;
        // Certificates would need a different login call; plain keys cover the common case.
        let keys: Vec<_> = identities
            .into_iter()
            .filter_map(|identity| match identity {
                AgentIdentity::PublicKey { key, .. } => Some(key),
                AgentIdentity::Certificate { .. } => None,
            })
            .collect();
        if keys.is_empty() {
            return Err(TunnelError::Agent("ssh-agent holds no usable keys".into()));
        }
        let count = keys.len();
        for key in keys {
            let hash = signing_hash(handle, key.algorithm().is_rsa()).await?;
            match handle
                .authenticate_publickey_with(&self.profile.username, key, hash, agent)
                .await
            {
                Ok(AuthResult::Success) => return Ok(()),
                Ok(AuthResult::Failure { .. }) => {}
                Err(_) if handle.is_closed() => {
                    return Err(TunnelError::Closed(
                        "the connection was lost during login".into(),
                    ));
                }
                Err(e) => self.reporter.warn(format!("ssh-agent did not sign: {e}")),
            }
        }
        Err(TunnelError::Auth(format!(
            "the server accepted none of the {count} keys in ssh-agent"
        )))
    }

    async fn attempts(&self, kind: SecretKind) -> Attempts<'_> {
        let first = match self.kept.take(kind) {
            Some(kept) => Some(kept),
            None => self.stored(kind).await,
        };
        Attempts {
            auth: self,
            kind,
            stored: first,
            tried: 0,
        }
    }

    async fn ask(&self, kind: SecretKind, retry: bool) -> Result<SecretReply, TunnelError> {
        let request = SecretRequest {
            request_id: String::new(),
            profile_id: self.profile.id.clone(),
            profile_name: self.profile.name.clone(),
            kind,
            retry,
        };
        tokio::select! {
            reply = self.env.interaction.request_secret(request) => reply.ok_or(TunnelError::Cancelled),
            _ = self.cancel.cancelled() => Err(TunnelError::Cancelled),
        }
    }

    async fn stored(&self, kind: SecretKind) -> Option<String> {
        let id = self.profile.id.clone();
        match blocking(&self.env.secrets, move |store| store.load(&id, kind)).await {
            Ok(secret) => secret,
            Err(e) => {
                self.reporter
                    .warn(format!("the system credential store is not available: {e}"));
                None
            }
        }
    }

    /// A secret that worked: kept in memory for the next login of this tunnel, and in the
    /// credential store as well if the user asked for that.
    async fn accepted(&self, kind: SecretKind, secret: Secret) {
        self.kept.keep(kind, secret.value.clone());
        if !secret.remember {
            return;
        }
        let id = self.profile.id.clone();
        let saved = blocking(&self.env.secrets, move |store| {
            store.store(&id, kind, &secret.value)
        })
        .await;
        if let Err(e) = saved {
            self.reporter.warn(format!(
                "could not save the {} in the system credential store: {e}",
                kind.as_str()
            ));
        }
    }
}

/// RSA keys must say which hash signs the login; ask the server which it accepts.
async fn signing_hash(
    handle: &Handle<ClientHandler>,
    is_rsa: bool,
) -> Result<Option<HashAlg>, TunnelError> {
    if !is_rsa {
        return Ok(None);
    }
    Ok(handle.best_supported_rsa_hash().await?.flatten())
}

/// Decrypting can take a second or more (the key derivation is slow on purpose).
async fn read_key(path: &Path, passphrase: Option<String>) -> Result<PrivateKey, keys::Error> {
    let path: PathBuf = path.to_owned();
    tokio::task::spawn_blocking(move || keys::load_secret_key(path, passphrase.as_deref()))
        .await
        .unwrap_or_else(|e| Err(keys::Error::IO(io::Error::other(e))))
}

fn key_file_error(path: &Path, error: &keys::Error) -> TunnelError {
    TunnelError::KeyFile(format!("{}: {error}", path.display()))
}

fn method_names(methods: &[MethodKind]) -> String {
    let names: Vec<&str> = methods.iter().map(<&str>::from).collect();
    names.join(", ")
}

#[cfg(unix)]
async fn connect_agent() -> Result<Agent, keys::Error> {
    Ok(AgentClient::connect_env().await?.dynamic())
}

/// The OpenSSH agent service listens on a named pipe; PuTTY users run Pageant instead.
#[cfg(windows)]
async fn connect_agent() -> Result<Agent, keys::Error> {
    let pipe =
        std::env::var_os("SSH_AUTH_SOCK").unwrap_or_else(|| r"\\.\pipe\openssh-ssh-agent".into());
    match AgentClient::connect_named_pipe(pipe).await {
        Ok(agent) => Ok(agent.dynamic()),
        Err(_) => Ok(AgentClient::connect_pageant().await?.dynamic()),
    }
}
