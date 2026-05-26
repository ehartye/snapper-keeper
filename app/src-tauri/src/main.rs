#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logging;

use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tracing::{error, info};

const TRAY_HOLO_PNG: &[u8] = include_bytes!("../icons/sk-holo.png");
const TRAY_MEMPHIS_PNG: &[u8] = include_bytes!("../icons/sk-memphis.png");
const TRAY_ROBOTIC_PNG: &[u8] = include_bytes!("../icons/sk-robotic.png");
const TRAY_CORPORATE_PNG: &[u8] = include_bytes!("../icons/sk-corporate.png");
const TRAY_WABI_SABI_PNG: &[u8] = include_bytes!("../icons/sk-wabi-sabi.png");
const TRAY_RISO_PNG: &[u8] = include_bytes!("../icons/sk-riso.png");
const TRAY_CONSTRUCTIVIST_PNG: &[u8] = include_bytes!("../icons/sk-constructivist.png");
const TRAY_ATOMIC_PNG: &[u8] = include_bytes!("../icons/sk-atomic.png");

fn tray_icon_for(family: &str) -> Image<'static> {
    let bytes = match family {
        "memphis" => TRAY_MEMPHIS_PNG,
        "robotic" => TRAY_ROBOTIC_PNG,
        "corporate" => TRAY_CORPORATE_PNG,
        "wabi-sabi" => TRAY_WABI_SABI_PNG,
        "riso" => TRAY_RISO_PNG,
        "constructivist" => TRAY_CONSTRUCTIVIST_PNG,
        "atomic" => TRAY_ATOMIC_PNG,
        _ => TRAY_HOLO_PNG,
    };
    let img = image::load_from_memory(bytes)
        .expect("tray icon png decode")
        .to_rgba8();
    let (w, h) = img.dimensions();
    Image::new_owned(img.into_raw(), w, h)
}

#[tauri::command]
fn set_tray_theme<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    family: String,
) -> Result<(), String> {
    let icon = tray_icon_for(&family);
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_icon(Some(icon.clone()))
            .map_err(|e| e.to_string())?;
    } else {
        // Expected during early startup before the tray is built; the next
        // theme apply (window focus / setting change) will succeed.
        tracing::debug!("set_tray_theme: tray 'main' not built yet");
    }

    // Also swap the per-window icon shown in the taskbar / titlebar / alt-tab,
    // so the live "app icon" matches the active theme. The bundled .exe icon
    // (set at build time via bundle.icon) is unaffected.
    for (label, win) in app.webview_windows() {
        if let Err(e) = win.set_icon(icon.clone()) {
            tracing::warn!(window = %label, error = %e, "failed to set window icon");
        }
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // Launch-at-login support. The plugin manages the platform-specific
        // bits (registry entry on Windows, LaunchAgent on macOS). The user
        // toggles it from Settings → Startup.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            // No extra args — the app reads its own state to decide whether
            // to start hidden or focused.
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(snk_library::init())
        .plugin(snk_hotkeys::init())
        .plugin(snk_capture::init())
        .plugin(snk_annotate::init())
        .plugin(snk_clipboard::init())
        .plugin(snk_ocr::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(snk_releaser::init())
        .invoke_handler(tauri::generate_handler![set_tray_theme])
        .setup(|app| {
            // Initialize file-based tracing (general + security logs +
            // panic-dump hook) per crates/snk-library/src/logging.rs.
            // The returned LoggingHandle holds tracing-appender's
            // background worker guards; leaking it keeps the workers
            // alive for the program's lifetime — the OS reaps them at
            // exit. (Tauri's app.manage requires Send+Sync; WorkerGuard
            // is Send but not Sync, so leak is simpler than wrapping.)
            let log_dir = app
                .path()
                .app_log_dir()
                .map_err(|e| format!("resolve log dir: {e}"))?;
            let logging_handle = logging::init(&log_dir)
                .map_err(|e| format!("init logging at {}: {e}", log_dir.display()))?;
            std::mem::forget(logging_handle);

            let capture_region = MenuItem::with_id(
                app,
                "tray:capture-region",
                "Capture region\tCtrl+Shift+4",
                true,
                None::<&str>,
            )?;
            let capture_window = MenuItem::with_id(
                app,
                "tray:capture-window",
                "Capture window\tCtrl+Shift+5",
                true,
                None::<&str>,
            )?;
            let capture_screen = MenuItem::with_id(
                app,
                "tray:capture-full-screen",
                "Capture screen\tCtrl+Shift+3",
                true,
                None::<&str>,
            )?;
            let capture_timed = MenuItem::with_id(
                app,
                "tray:capture-timed",
                "Timed (5s)\tCtrl+Shift+6",
                true,
                None::<&str>,
            )?;
            let clipboard_hist = MenuItem::with_id(
                app,
                "tray:clipboard-history",
                "Clipboard history\tCtrl+Shift+V",
                true,
                None::<&str>,
            )?;
            let sep = PredefinedMenuItem::separator(app)?;
            let open_lib =
                MenuItem::with_id(app, "tray:open-library", "Open library", true, None::<&str>)?;
            let settings =
                MenuItem::with_id(app, "tray:settings", "Settings…", true, None::<&str>)?;
            let check_update = MenuItem::with_id(
                app,
                "tray:check-update",
                "Check for updates",
                true,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &capture_region,
                    &capture_window,
                    &capture_screen,
                    &capture_timed,
                    &clipboard_hist,
                    &sep,
                    &open_lib,
                    &settings,
                    &check_update,
                    &quit,
                ],
            )?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon_for("holo"))
                .icon_as_template(false)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "tray:capture-full-screen" => {
                        let _ = app.emit("hotkey:capture-full-screen", ());
                    }
                    "tray:capture-region" => {
                        let _ = app.emit("hotkey:capture-region", ());
                    }
                    "tray:capture-window" => {
                        let _ = app.emit("hotkey:capture-window", ());
                    }
                    "tray:capture-timed" => {
                        let _ = app.emit("hotkey:capture-timed", ());
                    }
                    "tray:clipboard-history" => {
                        let _ = app.emit("hotkey:clipboard-history", ());
                    }
                    "tray:open-library" => {
                        if let Some(win) = app.get_webview_window("library") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "tray:settings" => {
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "tray:check-update" => {
                        let handle = app.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = snk_releaser::plugin::check_for_update(handle).await;
                        });
                    }
                    "tray:quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(win) = tray.app_handle().get_webview_window("library") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Defer window show until after the runtime + tray are online.
            // On Windows, showing during synchronous setup races Win32
            // window-station init, causing WebView2 "Invalid window handle"
            // and breaking IPC bootstrap injection.
            if let Some(win) = app.get_webview_window("library") {
                let _ = win.show();
                let _ = win.set_focus();
            }

            info!("snapper-keeper started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!(error = %e, "tauri runtime exited");
        });
}
