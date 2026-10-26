//! Configuration and runtime-state types shared by the engine, the Tauri layer and the UI.
//! Anything that crosses the IPC boundary is serialised as camelCase JSON.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::util::new_id;

pub type ProfileId = String;

fn default_ssh_port() -> u16 {
    22
}

fn default_loopback() -> String {
    "127.0.0.1".to_string()
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------------------------
// Configuration (persisted)
// ---------------------------------------------------------------------------------------------

/// One SSH server together with the ports forwarded through it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default)]
    pub id: ProfileId,
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    pub auth: Auth,
    #[serde(default)]
    pub forwards: Vec<Forward>,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    #[serde(default)]
    pub connect_on_launch: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Auth {
    /// Password, kept in the OS keychain or asked for on connect.
    Password,
    /// Private key file; the passphrase (if any) is handled like a password.
    Key { path: String },
    /// Keys held by ssh-agent (OpenSSH agent or Pageant).
    Agent,
}

impl Auth {
    /// The one kind of secret this method may ask for or remember, if any.
    pub fn secret_kind(&self) -> Option<SecretKind> {
        match self {
            Auth::Password => Some(SecretKind::Password),
            Auth::Key { .. } => Some(SecretKind::Passphrase),
            Auth::Agent => None,
        }
    }
}

/// `local bind_host:local_port  ->  remote_host:remote_port` (as seen from the server).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forward {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    /// Local address to listen on. `127.0.0.1` keeps the port private to this machine.
    #[serde(default = "default_loopback")]
    pub bind_host: String,
    pub local_port: u16,
    /// Target as seen from the server (`127.0.0.1` means the server itself).
    #[serde(default = "default_loopback")]
    pub remote_host: String,
    pub remote_port: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Closing the window hides it to the tray instead of quitting.
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, thiserror::Error)]
#[error("{code}: {message}")]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    /// Stable machine-readable code the UI maps to a localised message.
    pub code: &'static str,
    pub message: String,
    /// Which forward the problem is about, when it is about one.
    pub forward_index: Option<usize>,
}

impl ValidationError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            forward_index: None,
        }
    }

    fn at(mut self, index: usize) -> Self {
        self.forward_index = Some(index);
        self
    }
}

fn is_unspecified(host: &str) -> bool {
    host.parse::<IpAddr>()
        .map(|ip| ip.is_unspecified())
        .unwrap_or(false)
}

impl Forward {
    fn normalize(&mut self) -> Result<(), ValidationError> {
        self.label = self.label.trim().to_string();
        self.bind_host = self.bind_host.trim().to_string();
        if self.bind_host.is_empty() || self.bind_host.eq_ignore_ascii_case("localhost") {
            self.bind_host = default_loopback();
        }
        if self.bind_host.parse::<IpAddr>().is_err() {
            return Err(ValidationError::new(
                "bindHost",
                "bind address must be an IP address",
            ));
        }
        self.remote_host = self.remote_host.trim().to_string();
        if self.remote_host.is_empty() {
            self.remote_host = default_loopback();
        }
        if self.remote_host.chars().any(char::is_whitespace) {
            return Err(ValidationError::new(
                "remoteHost",
                "target host must not contain spaces",
            ));
        }
        if self.local_port == 0 {
            return Err(ValidationError::new("localPort", "local port is required"));
        }
        if self.remote_port == 0 {
            return Err(ValidationError::new(
                "remotePort",
                "remote port is required",
            ));
        }
        if self.id.is_empty() {
            self.id = new_id();
        }
        Ok(())
    }
}

impl Profile {
    /// Whether a tunnel started from `self` has to be restarted to match `other`. Only the name and
    /// `connect_on_launch` can change under a running tunnel.
    pub fn needs_restart_for(&self, other: &Profile) -> bool {
        let essentials = |p: &Profile| Profile {
            name: String::new(),
            connect_on_launch: false,
            ..p.clone()
        };
        essentials(self) != essentials(other)
    }

    /// Whether both reach the same account on the same server, whatever they are called.
    pub fn is_same_server(&self, other: &Profile) -> bool {
        self.host.eq_ignore_ascii_case(&other.host)
            && self.port == other.port
            && self.username == other.username
    }

    /// Trims and defaults the fields, assigns ids and rejects unusable profiles.
    pub fn normalize(&mut self) -> Result<(), ValidationError> {
        self.name = self.name.trim().to_string();
        self.host = self.host.trim().to_string();
        self.username = self.username.trim().to_string();

        if self.host.is_empty() || self.host.chars().any(char::is_whitespace) {
            return Err(ValidationError::new("host", "server address is required"));
        }
        if self.port == 0 {
            return Err(ValidationError::new("port", "SSH port is required"));
        }
        if self.username.is_empty() {
            return Err(ValidationError::new("username", "user name is required"));
        }
        if let Auth::Key { path } = &mut self.auth {
            *path = path.trim().to_string();
            if path.is_empty() {
                return Err(ValidationError::new(
                    "keyPath",
                    "private key path is required",
                ));
            }
        }
        if self.forwards.is_empty() {
            return Err(ValidationError::new(
                "forwards",
                "at least one port mapping is required",
            ));
        }
        for (index, forward) in self.forwards.iter_mut().enumerate() {
            forward.normalize().map_err(|e| e.at(index))?;
        }
        for (i, a) in self.forwards.iter().enumerate() {
            for (j, b) in self.forwards.iter().enumerate().skip(i + 1) {
                let same_interface = a.bind_host == b.bind_host
                    || is_unspecified(&a.bind_host)
                    || is_unspecified(&b.bind_host);
                if a.local_port == b.local_port && same_interface {
                    return Err(ValidationError::new(
                        "duplicatePort",
                        "two mappings use the same local port",
                    )
                    .at(j));
                }
            }
        }

        if self.name.is_empty() {
            self.name = self.host.clone();
        }
        if self.id.is_empty() {
            self.id = new_id();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// Runtime state (never persisted)
// ---------------------------------------------------------------------------------------------

/// A failure the UI can localise: `code` is stable, `detail` is raw technical text.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorInfo {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ConnState {
    Idle,
    Connecting,
    Connected,
    /// The connection is down and will be retried at `retry_at` (unix ms).
    Reconnecting {
        attempt: u32,
        retry_at: u64,
        error: ErrorInfo,
    },
    /// Gave up: retrying would not help (wrong password, rejected host key, ...).
    Failed {
        error: ErrorInfo,
    },
}

impl ConnState {
    /// Up, on its way up or on its way back up: what a "disconnect" would stop.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ConnState::Connecting | ConnState::Connected | ConnState::Reconnecting { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ForwardState {
    Pending,
    Listening,
    Failed { error: ErrorInfo },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardRuntime {
    pub id: String,
    pub state: ForwardState,
    /// Connections currently open through this mapping.
    pub active: u32,
    /// Connections served since the SSH session was established.
    pub total: u64,
    /// Bytes sent to the server / received from the server.
    pub bytes_up: u64,
    pub bytes_down: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRuntime {
    pub profile_id: ProfileId,
    pub conn: ConnState,
    pub forwards: Vec<ForwardRuntime>,
    /// Unix ms at which the current SSH session was established.
    pub connected_at: Option<u64>,
    /// Grows with every change. Updates can arrive out of order (and a snapshot can race live
    /// events), so the UI keeps whichever has the higher `rev`.
    pub rev: u64,
}

impl ForwardRuntime {
    pub fn new(id: &str, state: ForwardState) -> Self {
        Self {
            id: id.to_string(),
            state,
            active: 0,
            total: 0,
            bytes_up: 0,
            bytes_down: 0,
        }
    }
}

impl ProfileRuntime {
    pub fn idle(profile_id: &str) -> Self {
        Self {
            profile_id: profile_id.to_string(),
            conn: ConnState::Idle,
            forwards: Vec::new(),
            connected_at: None,
            rev: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// Position in the log of the whole app; orders lines and lets the UI drop duplicates.
    pub seq: u64,
    pub profile_id: ProfileId,
    pub ts: u64,
    pub level: LogLevel,
    pub message: String,
}

// ---------------------------------------------------------------------------------------------
// Questions the engine puts to the user
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretKind {
    Password,
    Passphrase,
}

impl SecretKind {
    pub const ALL: [SecretKind; 2] = [SecretKind::Password, SecretKind::Passphrase];

    pub fn as_str(self) -> &'static str {
        match self {
            SecretKind::Password => "password",
            SecretKind::Passphrase => "passphrase",
        }
    }
}

/// "This server presented a key we have never seen. Trust it?"
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyRequest {
    pub request_id: String,
    pub profile_id: ProfileId,
    pub profile_name: String,
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint: String,
    /// The server is already known by a key of another type.
    pub other_known: bool,
}

/// "Enter the password / key passphrase."
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRequest {
    pub request_id: String,
    pub profile_id: ProfileId,
    pub profile_name: String,
    pub kind: SecretKind,
    /// The previous attempt was rejected.
    pub retry: bool,
}

/// Everything the engine tells the outside world. One enum, one IPC event channel.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Event {
    State(ProfileRuntime),
    Log(LogLine),
    HostKey(HostKeyRequest),
    Secret(SecretRequest),
    /// A pending question is no longer relevant (answered, timed out or abandoned).
    PromptClosed {
        request_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forward(local: u16, remote: u16) -> Forward {
        Forward {
            id: String::new(),
            label: String::new(),
            bind_host: String::new(),
            local_port: local,
            remote_host: String::new(),
            remote_port: remote,
        }
    }

    fn profile() -> Profile {
        Profile {
            id: String::new(),
            name: "  ".into(),
            host: " example.com ".into(),
            port: 22,
            username: " root ".into(),
            auth: Auth::Agent,
            forwards: vec![forward(8080, 80)],
            auto_reconnect: true,
            connect_on_launch: false,
        }
    }

    #[test]
    fn normalize_fills_defaults_and_ids() {
        let mut p = profile();
        p.normalize().unwrap();
        assert_eq!(p.host, "example.com");
        assert_eq!(p.username, "root");
        assert_eq!(
            p.name, "example.com",
            "an empty name falls back to the host"
        );
        assert!(!p.id.is_empty());
        assert!(!p.forwards[0].id.is_empty());
        assert_eq!(p.forwards[0].bind_host, "127.0.0.1");
        assert_eq!(p.forwards[0].remote_host, "127.0.0.1");
    }

    #[test]
    fn localhost_is_stored_as_loopback_ip() {
        let mut p = profile();
        p.forwards[0].bind_host = "LocalHost".into();
        p.normalize().unwrap();
        assert_eq!(p.forwards[0].bind_host, "127.0.0.1");
    }

    #[test]
    fn rejects_unusable_profiles() {
        type Mutation = Box<dyn Fn(&mut Profile)>;
        let cases: Vec<(Mutation, &str)> = vec![
            (Box::new(|p| p.host = "  ".into()), "host"),
            (Box::new(|p| p.host = "a b".into()), "host"),
            (Box::new(|p| p.port = 0), "port"),
            (Box::new(|p| p.username = String::new()), "username"),
            (
                Box::new(|p| p.auth = Auth::Key { path: " ".into() }),
                "keyPath",
            ),
            (Box::new(|p| p.forwards.clear()), "forwards"),
            (Box::new(|p| p.forwards[0].local_port = 0), "localPort"),
            (Box::new(|p| p.forwards[0].remote_port = 0), "remotePort"),
            (
                Box::new(|p| p.forwards[0].bind_host = "not-an-ip".into()),
                "bindHost",
            ),
        ];
        for (mutate, code) in cases {
            let mut p = profile();
            mutate(&mut p);
            let err = p.normalize().unwrap_err();
            assert_eq!(err.code, code);
        }
    }

    #[test]
    fn duplicate_local_ports_are_reported_with_their_index() {
        let mut p = profile();
        p.forwards.push(forward(9090, 90));
        p.forwards.push(forward(8080, 81));
        let err = p.normalize().unwrap_err();
        assert_eq!(err.code, "duplicatePort");
        assert_eq!(err.forward_index, Some(2));

        // 0.0.0.0 overlaps every interface, so the same port is a conflict there too.
        let mut p = profile();
        let mut wide = forward(8080, 81);
        wide.bind_host = "0.0.0.0".into();
        p.forwards.push(wide);
        assert_eq!(p.normalize().unwrap_err().code, "duplicatePort");

        // Same port on different concrete interfaces is fine.
        let mut p = profile();
        let mut other = forward(8080, 81);
        other.bind_host = "127.0.0.2".into();
        p.forwards.push(other);
        p.normalize().unwrap();
    }

    #[test]
    fn only_connection_settings_restart_a_running_tunnel() {
        let mut before = profile();
        before.normalize().unwrap();
        let mut renamed = before.clone();
        renamed.name = "other".into();
        renamed.connect_on_launch = true;
        assert!(!before.needs_restart_for(&renamed));

        let edits: [fn(&mut Profile); 5] = [
            |p| p.host = "elsewhere".into(),
            |p| p.username = "you".into(),
            |p| p.auth = Auth::Password,
            |p| p.forwards[0].remote_port = 81,
            |p| p.auto_reconnect = false,
        ];
        for edit in edits {
            let mut after = before.clone();
            edit(&mut after);
            assert!(before.needs_restart_for(&after));
        }
    }

    #[test]
    fn a_server_is_the_same_by_host_user_and_port() {
        let mut before = profile();
        before.normalize().unwrap();
        let mut renamed = before.clone();
        renamed.name = "other".into();
        renamed.auth = Auth::Password;
        renamed.host = "EXAMPLE.com".into();
        assert!(before.is_same_server(&renamed));

        let edits: [fn(&mut Profile); 3] = [
            |p| p.host = "elsewhere.example.com".into(),
            |p| p.port = 2222,
            |p| p.username = "you".into(),
        ];
        for edit in edits {
            let mut after = before.clone();
            edit(&mut after);
            assert!(!before.is_same_server(&after));
        }
    }

    #[test]
    fn each_auth_method_asks_for_its_own_kind_of_secret() {
        assert_eq!(Auth::Password.secret_kind(), Some(SecretKind::Password));
        let key = Auth::Key { path: "k".into() };
        assert_eq!(key.secret_kind(), Some(SecretKind::Passphrase));
        assert_eq!(Auth::Agent.secret_kind(), None);
    }

    #[test]
    fn json_shape_is_what_the_ui_expects() {
        let mut p = profile();
        p.auth = Auth::Key {
            path: "~/.ssh/id_ed25519".into(),
        };
        p.normalize().unwrap();
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["auth"]["type"], "key");
        assert_eq!(json["auth"]["path"], "~/.ssh/id_ed25519");
        assert_eq!(json["autoReconnect"], true);
        assert_eq!(json["forwards"][0]["localPort"], 8080);

        let runtime = ProfileRuntime {
            profile_id: "p".into(),
            conn: ConnState::Reconnecting {
                attempt: 2,
                retry_at: 5,
                error: ErrorInfo {
                    code: "network".into(),
                    detail: "refused".into(),
                },
            },
            forwards: vec![],
            connected_at: None,
            rev: 7,
        };
        let json = serde_json::to_value(Event::State(runtime)).unwrap();
        assert_eq!(json["type"], "state");
        assert_eq!(json["profileId"], "p");
        assert_eq!(json["rev"], 7);
        assert_eq!(json["conn"]["state"], "reconnecting");
        assert_eq!(json["conn"]["retryAt"], 5);
        assert_eq!(json["conn"]["error"]["code"], "network");

        let json = serde_json::to_value(Event::PromptClosed {
            request_id: "r".into(),
        })
        .unwrap();
        assert_eq!(json["type"], "promptClosed");
        assert_eq!(json["requestId"], "r");
    }

    #[test]
    fn stored_profiles_without_optional_fields_still_load() {
        let json = r#"{
            "name": "x", "host": "h", "username": "u",
            "auth": { "type": "password" },
            "forwards": [{ "localPort": 1, "remotePort": 2 }]
        }"#;
        let p: Profile = serde_json::from_str(json).unwrap();
        assert_eq!(p.port, 22);
        assert!(p.auto_reconnect);
        assert!(!p.connect_on_launch);
        assert_eq!(p.forwards[0].bind_host, "127.0.0.1");
    }
}
