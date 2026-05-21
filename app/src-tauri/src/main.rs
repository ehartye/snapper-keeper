#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SNK_LOG").unwrap_or_else(|_| EnvFilter::new("info,snk=debug")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(snk_library::init())
        .plugin(snk_hotkeys::init())
        .plugin(snk_capture::init())
        .plugin(snk_annotate::init())
        .plugin(snk_clipboard::init())
        .plugin(snk_ocr::init())
        .setup(|app| {
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
                    &quit,
                ],
            )?;

            let _tray = TrayIconBuilder::with_id("main")
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
