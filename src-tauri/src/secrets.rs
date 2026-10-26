//! Passwords and key passphrases in the operating system's credential store
//! (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux).
//!
//! The keyring calls are blocking, so they run on the blocking thread pool.

use crate::model::SecretKind;

const SERVICE: &str = "com.chensy.server-ports";

fn account(profile_id: &str, kind: SecretKind) -> String {
    format!("{profile_id}:{}", kind.as_str())
}

async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|e| e.to_string())?
}

fn entry(profile_id: &str, kind: SecretKind) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, &account(profile_id, kind)).map_err(|e| e.to_string())
}

/// `Ok(None)` means "nothing stored"; `Err` means the credential store itself is unavailable.
pub async fn load(profile_id: &str, kind: SecretKind) -> Result<Option<String>, String> {
    let profile_id = profile_id.to_string();
    blocking(move || match entry(&profile_id, kind)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    })
    .await
}

pub async fn store(profile_id: &str, kind: SecretKind, secret: &str) -> Result<(), String> {
    let profile_id = profile_id.to_string();
    let secret = secret.to_string();
    blocking(move || {
        entry(&profile_id, kind)?
            .set_password(&secret)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Deleting something that is not there is not an error.
pub async fn remove(profile_id: &str, kind: SecretKind) -> Result<(), String> {
    let profile_id = profile_id.to_string();
    blocking(move || match entry(&profile_id, kind)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    })
    .await
}

pub async fn remove_all(profile_id: &str) {
    for kind in [SecretKind::Password, SecretKind::Passphrase] {
        let _ = remove(profile_id, kind).await;
    }
}
