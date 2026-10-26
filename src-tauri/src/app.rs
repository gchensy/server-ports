//! Wires the core to Tauri: plugins, the window, the tray, the event channel to the UI and a
//! shutdown that closes every listener.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};
use tokio::sync::Notify;

use crate::backend::Backend;
use crate::commands;
use crate::events::EventSink;
use crate::known_hosts::KnownHosts;
use crate::model::Event;
use crate::secrets::KeyringStore;
use crate::ssh::Env;
use crate::store::Store;
use crate::tray::Tray;

/// Passed by the autostart entry: start in the tray without showing the window.
const HIDDEN_FLAG: &str = "--hidden";
const MAIN_WINDOW: &str = "main";
/// The one channel the UI listens on for everything the engine reports.
const EVENT_CHANNEL: &str = "engine";
/// If the UI has not said it is ready by then, show the window anyway.
const READY_GRACE: Duration = Duration::from_secs(5);
/// Closing the listeners should take moments; this only keeps a stuck connection from holding up
/// the exit.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin: a second launch ends up here instead of in a second app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg(HIDDEN_FLAG)
                .build(),
        )
        .plugin(logging())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::save_profile,
            commands::delete_profile,
            commands::connect,
            commands::disconnect,
            commands::answer_host_key,
            commands::answer_secret,
            commands::saved_secrets,
            commands::forget_secrets,
            commands::forget_host_key,
            commands::set_settings,
            commands::autostart_enabled,
            commands::set_autostart,
            commands::set_tray_labels,
            commands::ui_ready,
            commands::quit,
        ])
        .on_window_event(on_window_event)
        .build(tauri::generate_context!())
        .expect("error while building the application")
        .run(on_run_event);
}

fn logging<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use log::LevelFilter::{Debug, Info, Warn};
    tauri_plugin_log::Builder::new()
        .level(if cfg!(debug_assertions) { Debug } else { Info })
        .level_for("russh", Warn)
        .level_for("tao", Warn)
        .level_for("wry", Warn)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
        .max_file_size(512 * 1024)
        .build()
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;

    let dirty = Arc::new(Notify::new());
    let sink = Arc::new(TauriSink {
        app: app.handle().clone(),
        dirty: dirty.clone(),
    });
    let env = Env::new(
        KnownHosts::load(dir.join("known_hosts.json")),
        KeyringStore,
        sink,
    );
    let backend = Arc::new(Backend::new(Store::load(dir.join("profiles.json")), env));
    app.manage(backend.clone());
    app.manage(Tray::install(app.handle(), backend.clone(), dirty)?);
    app.manage(Launch::new(std::env::args().any(|arg| arg == HIDDEN_FLAG)));
    app.manage(Quitting(AtomicBool::new(false)));

    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(READY_GRACE).await;
        handle.state::<Launch>().reveal(&handle);
    });
    tauri::async_runtime::spawn(async move { backend.connect_on_launch().await });
    Ok(())
}

/// Hides to the tray instead of closing, if the settings say so.
fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event
        && window.label() == MAIN_WINDOW
        && window.state::<Arc<Backend>>().settings().close_to_tray
    {
        api.prevent_close();
        let _ = window.hide();
    }
}

fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { api, .. } => {
            // The first request only starts the shutdown; the exit it ends with passes.
            if app.state::<Quitting>().0.swap(true, SeqCst) {
                return;
            }
            api.prevent_exit();
            let app = app.clone();
            let backend = app.state::<Arc<Backend>>().inner().clone();
            tauri::async_runtime::spawn(async move {
                let _ = tokio::time::timeout(SHUTDOWN_GRACE, backend.disconnect_all()).await;
                app.exit(0);
            });
        }
        // A click on the Dock icon while the window is hidden in the tray.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main_window(app),
        _ => {}
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

struct Quitting(AtomicBool);

/// Whether the window is to appear by itself when the UI has loaded. It is created hidden so that
/// nothing flashes before the first frame is painted.
pub struct Launch {
    hidden: bool,
    revealed: AtomicBool,
}

impl Launch {
    fn new(hidden: bool) -> Self {
        Self {
            hidden,
            revealed: AtomicBool::new(false),
        }
    }

    /// Shows the window the first time, unless the app was asked to start in the tray.
    pub fn reveal(&self, app: &AppHandle) {
        if !self.hidden && !self.revealed.swap(true, SeqCst) {
            log::debug!("showing the main window");
            show_main_window(app);
        }
    }
}

/// Carries what the engine reports to the UI, and takes care of what the window and the tray
/// have to do about it.
struct TauriSink {
    app: AppHandle,
    dirty: Arc<Notify>,
}

impl EventSink for TauriSink {
    fn emit(&self, event: Event) {
        match &event {
            Event::State(_) => self.dirty.notify_one(),
            // Somebody has to answer, so the window comes forward.
            Event::HostKey(_) | Event::Secret(_) => {
                let app = self.app.clone();
                let _ = self.app.run_on_main_thread(move || show_main_window(&app));
            }
            Event::Log(_) | Event::PromptClosed { .. } => {}
        }
        if let Err(e) = self.app.emit(EVENT_CHANNEL, &event) {
            log::warn!("could not send an event to the window: {e}");
        }
    }
}
