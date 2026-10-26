//! Trust-on-first-use store for server host keys (`known_hosts.json` in the app data directory).
//!
//! A server is identified by `host:port`. Per server we remember one key per key *family*
//! (ed25519, rsa, ecdsa-...), the way OpenSSH keeps one line per key type: a server that is
//! known by its ed25519 key and then shows a *different* ed25519 key is an alarm; a server that
//! shows an RSA key we have never seen merely asks again.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use russh::keys::{Algorithm, HashAlg, PublicKey};
use serde::{Deserialize, Serialize};

use crate::util::{now_ms, write_atomic};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnownKey {
    family: String,
    /// The key in OpenSSH one-line format, for humans and for debugging.
    key: String,
    fingerprint: String,
    added_at: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileData {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    hosts: BTreeMap<String, Vec<KnownKey>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostKeyCheck {
    /// This exact key is on file.
    Trusted,
    /// Nothing on file for this key type. `other_known` says whether the server is known by another type.
    Unknown { other_known: bool },
    /// A *different* key of the same type is on file: possibly a man-in-the-middle.
    Changed { known_fingerprint: String },
}

/// Key type as shown to the user and used as the per-server key slot (`ssh-rsa` covers every RSA hash).
pub fn key_family(algorithm: &Algorithm) -> String {
    match algorithm {
        Algorithm::Rsa { .. } => "ssh-rsa".to_string(),
        other => other.as_str().to_string(),
    }
}

/// `SHA256:...`, byte for byte what `ssh-keygen -l` prints.
pub fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

pub struct KnownHosts {
    path: Option<PathBuf>,
    hosts: Mutex<BTreeMap<String, Vec<KnownKey>>>,
}

impl KnownHosts {
    /// A missing or unreadable file is an empty store (the user is simply asked again).
    pub fn load(path: PathBuf) -> Self {
        let hosts = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<FileData>(&bytes) {
                Ok(data) => data.hosts,
                Err(e) => {
                    log::error!("{} is not valid ({e}); starting empty", path.display());
                    BTreeMap::new()
                }
            },
            Err(_) => BTreeMap::new(),
        };
        Self {
            path: Some(path),
            hosts: Mutex::new(hosts),
        }
    }

    /// Nothing is persisted (tests).
    pub fn in_memory() -> Self {
        Self {
            path: None,
            hosts: Mutex::new(BTreeMap::new()),
        }
    }

    fn id(host: &str, port: u16) -> String {
        format!("{}:{port}", host.to_ascii_lowercase())
    }

    pub fn check(&self, host: &str, port: u16, key: &PublicKey) -> HostKeyCheck {
        let family = key_family(&key.algorithm());
        let hosts = self.hosts.lock().unwrap();
        let Some(known) = hosts.get(&Self::id(host, port)).filter(|k| !k.is_empty()) else {
            return HostKeyCheck::Unknown { other_known: false };
        };
        match known.iter().find(|k| k.family == family) {
            Some(k) if k.fingerprint == fingerprint(key) => HostKeyCheck::Trusted,
            Some(k) => HostKeyCheck::Changed {
                known_fingerprint: k.fingerprint.clone(),
            },
            None => HostKeyCheck::Unknown { other_known: true },
        }
    }

    /// Remembers `key` for the server, replacing a key of the same type. In-memory trust is kept
    /// even when persisting fails, so the connection in progress is not affected.
    pub fn trust(&self, host: &str, port: u16, key: &PublicKey) -> std::io::Result<()> {
        let family = key_family(&key.algorithm());
        let entry = KnownKey {
            family: family.clone(),
            key: key.to_openssh().unwrap_or_default(),
            fingerprint: fingerprint(key),
            added_at: now_ms(),
        };
        {
            let mut hosts = self.hosts.lock().unwrap();
            let list = hosts.entry(Self::id(host, port)).or_default();
            list.retain(|k| k.family != family);
            list.push(entry);
        }
        self.persist()
    }

    /// Forgets everything known about the server. Returns whether there was anything.
    pub fn forget(&self, host: &str, port: u16) -> std::io::Result<bool> {
        let removed = self
            .hosts
            .lock()
            .unwrap()
            .remove(&Self::id(host, port))
            .is_some();
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    /// Key types on file for the server, to be offered first during the handshake so that a
    /// server with several host keys presents the one we already trust.
    pub fn known_families(&self, host: &str, port: u16) -> Vec<String> {
        self.hosts
            .lock()
            .unwrap()
            .get(&Self::id(host, port))
            .map(|keys| keys.iter().map(|k| k.family.clone()).collect())
            .unwrap_or_default()
    }

    fn persist(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let data = FileData {
            version: 1,
            hosts: self.hosts.lock().unwrap().clone(),
        };
        let bytes = serde_json::to_vec_pretty(&data).expect("known hosts are serialisable");
        write_atomic(path, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Generated with `ssh-keygen -t ed25519`; the fingerprints are what `ssh-keygen -lf` printed.
    const KEY_A: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEtnDHAWvEDBvHF7a+BYC5waMR+D6DGw5jH6GDKRoHPG";
    const FP_A: &str = "SHA256:LqZdrggkSbuaka1XCAbHe3DF8Y/gDc61ySTihzTg9kk";
    const KEY_B: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINdRfmE2jQsLlIwS67vK6grL2wEL2QXIihvIxjn+hKw8";
    const FP_B: &str = "SHA256:ymTTg6ZtSFVcXgiP6k5r6nz2AKjXj7eomQxuqgBIXjQ";

    fn key(openssh: &str) -> PublicKey {
        PublicKey::from_openssh(openssh).unwrap()
    }

    #[test]
    fn fingerprints_match_openssh() {
        assert_eq!(fingerprint(&key(KEY_A)), FP_A);
        assert_eq!(fingerprint(&key(KEY_B)), FP_B);
        assert_eq!(key_family(&key(KEY_A).algorithm()), "ssh-ed25519");
    }

    #[test]
    fn first_sight_then_trusted_then_changed() {
        let known = KnownHosts::in_memory();
        assert_eq!(
            known.check("Example.COM", 22, &key(KEY_A)),
            HostKeyCheck::Unknown { other_known: false }
        );
        known.trust("example.com", 22, &key(KEY_A)).unwrap();
        // Host names are case-insensitive; ports are part of the identity.
        assert_eq!(
            known.check("EXAMPLE.com", 22, &key(KEY_A)),
            HostKeyCheck::Trusted
        );
        assert_eq!(
            known.check("example.com", 2222, &key(KEY_A)),
            HostKeyCheck::Unknown { other_known: false }
        );
        assert_eq!(
            known.check("example.com", 22, &key(KEY_B)),
            HostKeyCheck::Changed {
                known_fingerprint: FP_A.to_string()
            }
        );
    }

    #[test]
    fn retrusting_replaces_the_key_of_the_same_type() {
        let known = KnownHosts::in_memory();
        known.trust("h", 22, &key(KEY_A)).unwrap();
        known.trust("h", 22, &key(KEY_B)).unwrap();
        assert_eq!(known.check("h", 22, &key(KEY_B)), HostKeyCheck::Trusted);
        assert_eq!(known.known_families("h", 22), ["ssh-ed25519"]);
    }

    #[test]
    fn forgetting_a_host_starts_over() {
        let known = KnownHosts::in_memory();
        known.trust("h", 22, &key(KEY_A)).unwrap();
        assert!(known.forget("H", 22).unwrap());
        assert!(!known.forget("h", 22).unwrap());
        assert_eq!(
            known.check("h", 22, &key(KEY_B)),
            HostKeyCheck::Unknown { other_known: false }
        );
    }

    #[test]
    fn trust_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("known_hosts.json");
        KnownHosts::load(path.clone())
            .trust("h", 22, &key(KEY_A))
            .unwrap();
        let reloaded = KnownHosts::load(path);
        assert_eq!(reloaded.check("h", 22, &key(KEY_A)), HostKeyCheck::Trusted);
    }
}
