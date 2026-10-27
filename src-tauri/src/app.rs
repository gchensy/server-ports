//! Wires the core to Tauri: plugins, the window, the tray, the event channel to the UI and a
//! shutdown that closes every listener.

use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::utils::config::WindowConfig;
use tauri::{
    AppHandle, Emitter, Manager, RunEvent, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
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
/// If the UI has not said it is ready by then, it is taken to be.
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
        .plugin(tauri_plugin_opener::init())
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
            commands::discover_ssh_hosts,
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
    // The window of the config is there to load the UI, which sends the tray its labels; the
    // autostart entry does not want it on screen, so it goes again once it has loaded.
    app.manage(MainWindow::new(
        std::env::args().any(|arg| arg == HIDDEN_FLAG),
    ));
    app.manage(Quitting(AtomicBool::new(false)));

    MainWindow::give_ui_time(app.handle());
    tauri::async_runtime::spawn(async move { backend.connect_on_launch().await });
    Ok(())
}

/// Closing the main window leaves the app in the tray if the settings say so, and ends it if not.
fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW {
        return;
    }
    let Some(main) = window.try_state::<MainWindow>() else {
        return;
    };
    match event {
        WindowEvent::CloseRequested { .. } => {
            if window.state::<Arc<Backend>>().settings().close_to_tray {
                main.remember_and_leave(window);
            } else {
                window.app_handle().exit(0);
            }
        }
        WindowEvent::Destroyed => main.destroyed(),
        _ => {}
    }
}

fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { api, code, .. } => {
            // The window that went was the last one, and the app lives on in the tray.
            if code.is_none() && app.state::<MainWindow>().was_left_in_tray() {
                api.prevent_exit();
                return;
            }
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

/// Brings the main window to the front; there may be none, and then a new one is made.
pub fn show_main_window(app: &AppHandle) {
    if let Some(main) = app.try_state::<MainWindow>() {
        main.open(app);
    }
}

fn show(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

struct Quitting(AtomicBool);

/// Where the main window is in its life. Its webview is most of what the app costs in memory
/// (WebKit runs processes of its own for it), so the window only exists while somebody wants it:
/// closing it to the tray destroys it, and the next request makes a new one. A new window is
/// created hidden and shown once its UI has painted, so that nothing flashes before the first frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Absent,
    /// The window is there and its UI is loading; `wanted`: it is to appear as soon as it has.
    Loading {
        wanted: bool,
    },
    /// The UI has painted: the window is on screen.
    Ready,
}

/// What a change of phase asks of the window.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    Nothing,
    Build,
    Show,
    Destroy,
}

impl Phase {
    /// Somebody wants the window on screen.
    fn open(&mut self) -> Step {
        match *self {
            Phase::Absent => {
                *self = Phase::Loading { wanted: true };
                Step::Build
            }
            Phase::Loading { .. } => {
                *self = Phase::Loading { wanted: true };
                Step::Nothing
            }
            Phase::Ready => Step::Show,
        }
    }

    /// The UI has painted its first frame, or is taken to have.
    fn ready(&mut self) -> Step {
        match *self {
            Phase::Loading { wanted: true } => {
                *self = Phase::Ready;
                Step::Show
            }
            Phase::Loading { wanted: false } => {
                *self = Phase::Absent;
                Step::Destroy
            }
            Phase::Ready | Phase::Absent => Step::Nothing,
        }
    }
}

/// Where the user left the window, in logical pixels, so that the next one opens there.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    maximized: bool,
    size: (f64, f64),
    position: Option<(f64, f64)>,
}

impl Placement {
    fn of(window: &tauri::Window) -> Option<Self> {
        let scale = window.scale_factor().ok()?;
        let size = window.inner_size().ok()?.to_logical::<f64>(scale);
        let position = window.outer_position().ok().map(|p| {
            let p = p.to_logical::<f64>(scale);
            (p.x, p.y)
        });
        Some(Self {
            maximized: window.is_maximized().unwrap_or(false),
            size: (size.width, size.height),
            position,
        })
    }

    fn apply(&self, config: &mut WindowConfig) {
        config.maximized = self.maximized;
        // A maximized window does not know the size it will be restored to; the config's will do.
        if self.maximized {
            return;
        }
        (config.width, config.height) = self.size;
        if let Some((x, y)) = self.position {
            (config.center, config.x, config.y) = (false, Some(x), Some(y));
        }
    }
}

/// The main window, which comes and goes (see `Phase`).
pub struct MainWindow {
    phase: Mutex<Phase>,
    placement: Mutex<Option<Placement>>,
    /// The window was closed to the tray: the exit its going would ask for is not wanted.
    left_in_tray: AtomicBool,
}

impl MainWindow {
    /// Tauri makes the window of the config by itself. `hidden`: nobody wants it on screen yet.
    fn new(hidden: bool) -> Self {
        Self {
            phase: Mutex::new(Phase::Loading { wanted: !hidden }),
            placement: Mutex::new(None),
            left_in_tray: AtomicBool::new(false),
        }
    }

    /// If the UI has not said it is ready after a while, it is taken to be: a page that never
    /// loads must not leave the app without a window.
    fn give_ui_time(app: &AppHandle) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(READY_GRACE).await;
            app.state::<MainWindow>().ready(&app);
        });
    }

    /// Brings the window to the front, making it first if there is none.
    fn open(&self, app: &AppHandle) {
        let window = app.get_webview_window(MAIN_WINDOW);
        let step = {
            let mut phase = self.phase.lock().unwrap();
            if window.is_none() {
                // Whatever the books say, there is nothing to wait for.
                *phase = Phase::Absent;
            }
            phase.open()
        };
        match (step, window) {
            (Step::Build, _) => self.build(app),
            (Step::Show, Some(window)) => show(&window),
            _ => {}
        }
    }

    /// Makes the window again, where the last one was, hidden until its UI is ready.
    fn build(&self, app: &AppHandle) {
        let placement = *self.placement.lock().unwrap();
        let app = app.clone();
        // Windows are made on the main thread, whoever asks for one.
        let _ = app
            .clone()
            .run_on_main_thread(move || match create_window(&app, placement) {
                Ok(()) => Self::give_ui_time(&app),
                Err(e) => {
                    log::error!("could not make the main window: {e}");
                    *app.state::<MainWindow>().phase.lock().unwrap() = Phase::Absent;
                }
            });
    }

    /// The UI has painted its first frame: the window may appear, or go if nobody wanted it.
    pub fn ready(&self, app: &AppHandle) {
        let step = self.phase.lock().unwrap().ready();
        let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
            return;
        };
        match step {
            Step::Show => {
                log::debug!("showing the main window");
                show(&window);
            }
            Step::Destroy => {
                log::debug!("nobody asked for the main window: destroying it");
                self.left_in_tray.store(true, SeqCst);
                // Not from inside the call the UI is still making.
                tauri::async_runtime::spawn(async move {
                    let _ = window.destroy();
                });
            }
            Step::Nothing | Step::Build => {}
        }
    }

    /// The window is being closed to the tray: the next one opens where this one is.
    fn remember_and_leave(&self, window: &tauri::Window) {
        *self.placement.lock().unwrap() = Placement::of(window);
        self.left_in_tray.store(true, SeqCst);
    }

    fn destroyed(&self) {
        *self.phase.lock().unwrap() = Phase::Absent;
    }

    /// Whether the window that has gone was left in the tray on purpose. Asked once per exit request.
    fn was_left_in_tray(&self) -> bool {
        self.left_in_tray.swap(false, SeqCst)
    }
}

fn create_window(app: &AppHandle, placement: Option<Placement>) -> tauri::Result<()> {
    let mut config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW)
        .cloned()
        .ok_or(tauri::Error::WindowNotFound)?;
    if let Some(placement) = placement {
        placement.apply(&mut config);
    }
    WebviewWindowBuilder::from_config(app, &config)?.build()?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_that_is_wanted_appears_once_its_ui_is_ready() {
        let mut phase = Phase::Loading { wanted: true };
        assert_eq!(phase.ready(), Step::Show);
        assert_eq!(phase, Phase::Ready);
    }

    #[test]
    fn a_window_nobody_wants_goes_once_its_ui_is_ready() {
        let mut phase = Phase::Loading { wanted: false };
        assert_eq!(phase.ready(), Step::Destroy);
        assert_eq!(phase, Phase::Absent);
    }

    #[test]
    fn asking_while_the_ui_loads_makes_the_window_appear() {
        let mut phase = Phase::Loading { wanted: false };
        assert_eq!(phase.open(), Step::Nothing);
        assert_eq!(phase.ready(), Step::Show);
    }

    #[test]
    fn a_window_that_went_is_made_again() {
        let mut phase = Phase::Absent;
        assert_eq!(phase.open(), Step::Build);
        assert_eq!(phase.open(), Step::Nothing);
        assert_eq!(phase.ready(), Step::Show);
        assert_eq!(phase.open(), Step::Show);
    }

    #[test]
    fn ready_means_nothing_when_there_is_nothing_to_show() {
        assert_eq!(Phase::Absent.ready(), Step::Nothing);
        assert_eq!(Phase::Ready.ready(), Step::Nothing);
    }

    #[test]
    fn a_new_window_opens_where_the_last_one_was() {
        let mut config = WindowConfig {
            center: true,
            ..Default::default()
        };
        Placement {
            maximized: false,
            size: (900.0, 640.0),
            position: Some((120.0, 80.0)),
        }
        .apply(&mut config);
        assert_eq!((config.width, config.height), (900.0, 640.0));
        assert_eq!(
            (config.x, config.y, config.center),
            (Some(120.0), Some(80.0), false)
        );
        assert!(!config.maximized);
    }

    #[test]
    fn a_maximized_window_keeps_the_size_of_the_config_to_come_back_to() {
        let mut config = WindowConfig::default();
        let (width, height) = (config.width, config.height);
        Placement {
            maximized: true,
            size: (1920.0, 1080.0),
            position: Some((0.0, 0.0)),
        }
        .apply(&mut config);
        assert!(config.maximized);
        assert_eq!((config.width, config.height), (width, height));
    }

    #[test]
    fn a_window_without_a_known_position_stays_centred() {
        let mut config = WindowConfig {
            center: true,
            ..Default::default()
        };
        Placement {
            maximized: false,
            size: (900.0, 640.0),
            position: None,
        }
        .apply(&mut config);
        assert!(config.center);
        assert_eq!((config.x, config.y), (None, None));
    }
}
