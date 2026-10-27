//! What the UI can ask for. Thin on purpose: the work is done by [`Backend`].

use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::app::MainWindow;
use crate::backend::{Backend, BackendError, DiscoveredHost, Snapshot};
use crate::interaction::SecretReply;
use crate::model::{Profile, SecretKind, Settings};
use crate::ssh_config::LocalUser;
use crate::tray::{Labels, Tray};

type Core<'a> = State<'a, Arc<Backend>>;

#[tauri::command]
pub fn snapshot(backend: Core) -> Snapshot {
    backend.snapshot()
}

#[tauri::command]
pub async fn save_profile(
    backend: Core<'_>,
    tray: State<'_, Arc<Tray>>,
    profile: Profile,
) -> Result<Profile, BackendError> {
    let saved = backend.save_profile(profile).await?;
    tray.invalidate();
    Ok(saved)
}

#[tauri::command]
pub async fn delete_profile(
    backend: Core<'_>,
    tray: State<'_, Arc<Tray>>,
    id: String,
) -> Result<(), BackendError> {
    backend.delete_profile(&id).await?;
    tray.invalidate();
    Ok(())
}

/// The servers of the user's `~/.ssh/config`; nothing is saved.
#[tauri::command]
pub fn discover_ssh_hosts(backend: Core) -> Result<Vec<DiscoveredHost>, BackendError> {
    backend.discover_ssh_hosts(&LocalUser::current())
}

#[tauri::command]
pub async fn connect(backend: Core<'_>, id: String) -> Result<(), BackendError> {
    backend.connect(&id).await
}

#[tauri::command]
pub async fn disconnect(backend: Core<'_>, id: String) -> Result<(), BackendError> {
    backend.disconnect(&id).await;
    Ok(())
}

/// `false`: the question is gone already.
#[tauri::command]
pub fn answer_host_key(backend: Core, request_id: String, trust: bool) -> bool {
    backend.answer_host_key(&request_id, trust)
}

/// `value == None` cancels the question. `false`: the question is gone already.
#[tauri::command]
pub fn answer_secret(
    backend: Core,
    request_id: String,
    value: Option<String>,
    remember: bool,
) -> bool {
    let reply = value.map(|value| SecretReply { value, remember });
    backend.answer_secret(&request_id, reply)
}

#[tauri::command]
pub async fn saved_secrets(backend: Core<'_>, id: String) -> Result<Vec<SecretKind>, BackendError> {
    backend.saved_secrets(&id).await
}

#[tauri::command]
pub async fn forget_secrets(backend: Core<'_>, id: String) -> Result<(), BackendError> {
    backend.forget_secrets(&id).await
}

#[tauri::command]
pub fn forget_host_key(backend: Core, id: String) -> Result<bool, BackendError> {
    backend.forget_host_key(&id)
}

#[tauri::command]
pub fn set_settings(backend: Core, settings: Settings) -> Result<(), BackendError> {
    backend.set_settings(settings)
}

#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    }
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_tray_labels(tray: State<'_, Arc<Tray>>, labels: Labels) {
    tray.set_labels(labels);
}

/// The UI has painted its first frame: time to show the window.
#[tauri::command]
pub fn ui_ready(app: AppHandle, window: State<'_, MainWindow>) {
    window.ready(&app);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
