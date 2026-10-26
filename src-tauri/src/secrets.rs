//! Passwords and key passphrases the user chose to remember.
//!
//! [`SecretStore`] is the seam: the app uses the operating system's credential store
//! ([`KeyringStore`]: Keychain on macOS, Credential Manager on Windows, Secret Service on Linux);
//! tests use [`MemoryStore`], so they never touch the real keychain.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::model::SecretKind;

const SERVICE: &str = "com.chensy.server-ports";

pub trait SecretStore: Send + Sync + 'static {
    /// `Ok(None)` means "nothing stored"; `Err` means the store itself is unavailable.
    fn load(&self, profile_id: &str, kind: SecretKind) -> Result<Option<String>, String>;
    fn store(&self, profile_id: &str, kind: SecretKind, secret: &str) -> Result<(), String>;
    /// Deleting something that is not there is not an error.
    fn remove(&self, profile_id: &str, kind: SecretKind) -> Result<(), String>;

    fn remove_all(&self, profile_id: &str) {
        for kind in SecretKind::ALL {
            let _ = self.remove(profile_id, kind);
        }
    }
}

/// Runs a store call on the blocking thread pool: the OS credential APIs may block, for example
/// while the keyring is waiting to be unlocked.
pub async fn blocking<T: Send + 'static>(
    store: &Arc<dyn SecretStore>,
    job: impl FnOnce(&dyn SecretStore) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let store = store.clone();
    tokio::task::spawn_blocking(move || job(&*store))
        .await
        .map_err(|e| e.to_string())?
}

fn account(profile_id: &str, kind: SecretKind) -> String {
    format!("{profile_id}:{}", kind.as_str())
}

pub struct KeyringStore;

impl KeyringStore {
    fn entry(profile_id: &str, kind: SecretKind) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, &account(profile_id, kind)).map_err(|e| e.to_string())
    }
}

impl SecretStore for KeyringStore {
    fn load(&self, profile_id: &str, kind: SecretKind) -> Result<Option<String>, String> {
        match Self::entry(profile_id, kind)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn store(&self, profile_id: &str, kind: SecretKind, secret: &str) -> Result<(), String> {
        Self::entry(profile_id, kind)?
            .set_password(secret)
            .map_err(|e| e.to_string())
    }

    fn remove(&self, profile_id: &str, kind: SecretKind) -> Result<(), String> {
        match Self::entry(profile_id, kind)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Keeps secrets in memory only.
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn load(&self, profile_id: &str, kind: SecretKind) -> Result<Option<String>, String> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .get(&account(profile_id, kind))
            .cloned())
    }

    fn store(&self, profile_id: &str, kind: SecretKind, secret: &str) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .insert(account(profile_id, kind), secret.to_string());
        Ok(())
    }

    fn remove(&self, profile_id: &str, kind: SecretKind) -> Result<(), String> {
        self.0.lock().unwrap().remove(&account(profile_id, kind));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_kept_per_profile_and_kind() {
        let store = MemoryStore::default();
        store.store("a", SecretKind::Password, "one").unwrap();
        store.store("a", SecretKind::Passphrase, "two").unwrap();
        assert_eq!(
            store.load("a", SecretKind::Password).unwrap().as_deref(),
            Some("one")
        );
        assert_eq!(store.load("b", SecretKind::Password).unwrap(), None);
        store.remove_all("a");
        assert_eq!(store.load("a", SecretKind::Password).unwrap(), None);
        assert_eq!(store.load("a", SecretKind::Passphrase).unwrap(), None);
    }

    #[tokio::test]
    async fn blocking_runs_the_job_against_the_store() {
        let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
        blocking(&store, |s| s.store("p", SecretKind::Password, "pw"))
            .await
            .unwrap();
        let loaded = blocking(&store, |s| s.load("p", SecretKind::Password)).await;
        assert_eq!(loaded.unwrap().as_deref(), Some("pw"));
    }
}
