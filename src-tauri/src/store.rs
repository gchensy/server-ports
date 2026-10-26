//! Profiles and settings on disk (`profiles.json` in the app data directory).

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::model::{Profile, Settings, ValidationError};
use crate::util::write_atomic;

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    profiles: Vec<Profile>,
    #[serde(default)]
    settings: Settings,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Invalid(#[from] ValidationError),
    #[error("could not write {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
}

pub struct Store {
    path: PathBuf,
    data: Mutex<Data>,
}

impl Store {
    /// Loads `path`. A missing file is an empty store; a corrupt file is moved aside
    /// (`profiles.json.corrupt`) rather than overwritten, so nothing the user typed is lost for good.
    pub fn load(path: PathBuf) -> Self {
        let data = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Data>(&bytes) {
                Ok(data) => data,
                Err(e) => {
                    log::error!("{} is not valid ({e}); starting empty", path.display());
                    let mut aside = path.as_os_str().to_owned();
                    aside.push(".corrupt");
                    let _ = std::fs::rename(&path, PathBuf::from(aside));
                    Data::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Data::default(),
            Err(e) => {
                log::error!("cannot read {}: {e}", path.display());
                Data::default()
            }
        };
        Self {
            path,
            data: Mutex::new(data),
        }
    }

    pub fn profiles(&self) -> Vec<Profile> {
        self.data.lock().unwrap().profiles.clone()
    }

    pub fn profile(&self, id: &str) -> Option<Profile> {
        self.data
            .lock()
            .unwrap()
            .profiles
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// Validates, then inserts or replaces (by id) and saves. Returns the normalised profile.
    pub fn upsert(&self, mut profile: Profile) -> Result<Profile, StoreError> {
        profile.normalize()?;
        let mut data = self.data.lock().unwrap();
        let mut next = data.clone();
        match next.profiles.iter_mut().find(|p| p.id == profile.id) {
            Some(existing) => *existing = profile.clone(),
            None => next.profiles.push(profile.clone()),
        }
        self.save(&next)?;
        *data = next;
        Ok(profile)
    }

    /// `Ok(false)` when there was no such profile.
    pub fn delete(&self, id: &str) -> Result<bool, StoreError> {
        let mut data = self.data.lock().unwrap();
        let mut next = data.clone();
        next.profiles.retain(|p| p.id != id);
        if next.profiles.len() == data.profiles.len() {
            return Ok(false);
        }
        self.save(&next)?;
        *data = next;
        Ok(true)
    }

    pub fn settings(&self) -> Settings {
        self.data.lock().unwrap().settings.clone()
    }

    pub fn set_settings(&self, settings: Settings) -> Result<(), StoreError> {
        let mut data = self.data.lock().unwrap();
        let mut next = data.clone();
        next.settings = settings;
        self.save(&next)?;
        *data = next;
        Ok(())
    }

    fn save(&self, data: &Data) -> Result<(), StoreError> {
        let mut data = data.clone();
        data.version = FORMAT_VERSION;
        let bytes = serde_json::to_vec_pretty(&data).expect("profiles are serialisable");
        write_atomic(&self.path, &bytes).map_err(|source| StoreError::Io {
            path: self.path.display().to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Auth, Forward};

    fn profile(name: &str) -> Profile {
        Profile {
            id: String::new(),
            name: name.into(),
            host: "example.com".into(),
            port: 22,
            username: "me".into(),
            auth: Auth::Agent,
            forwards: vec![Forward {
                id: String::new(),
                label: String::new(),
                bind_host: String::new(),
                local_port: 8080,
                remote_host: String::new(),
                remote_port: 80,
            }],
            auto_reconnect: true,
            connect_on_launch: false,
        }
    }

    #[test]
    fn profiles_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");
        let saved = {
            let store = Store::load(path.clone());
            let saved = store.upsert(profile("one")).unwrap();
            store.upsert(profile("two")).unwrap();
            store
                .set_settings(Settings {
                    close_to_tray: false,
                })
                .unwrap();
            saved
        };
        let store = Store::load(path);
        assert_eq!(store.profiles().len(), 2);
        assert_eq!(store.profile(&saved.id), Some(saved));
        assert!(!store.settings().close_to_tray);
    }

    #[test]
    fn upsert_replaces_by_id_and_keeps_order() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().join("profiles.json"));
        let first = store.upsert(profile("one")).unwrap();
        store.upsert(profile("two")).unwrap();
        let mut edited = first.clone();
        edited.name = "renamed".into();
        store.upsert(edited).unwrap();
        let names: Vec<_> = store.profiles().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["renamed", "two"]);
    }

    #[test]
    fn invalid_profiles_are_rejected_and_nothing_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");
        let store = Store::load(path.clone());
        let mut bad = profile("bad");
        bad.host = String::new();
        assert!(matches!(store.upsert(bad), Err(StoreError::Invalid(_))));
        assert!(store.profiles().is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn delete_reports_whether_anything_was_removed() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().join("profiles.json"));
        let saved = store.upsert(profile("one")).unwrap();
        assert!(store.delete(&saved.id).unwrap());
        assert!(!store.delete(&saved.id).unwrap());
    }

    #[test]
    fn a_corrupt_file_is_moved_aside_not_destroyed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");
        std::fs::write(&path, b"{ definitely not json").unwrap();
        let store = Store::load(path.clone());
        assert!(store.profiles().is_empty());
        let aside = dir.path().join("profiles.json.corrupt");
        assert_eq!(std::fs::read(aside).unwrap(), b"{ definitely not json");
    }
}
