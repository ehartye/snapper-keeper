#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
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
        .setup(|app| {
            // Build tray menu
            let capture_item = MenuItem::with_id(
                app,
                "tray:capture-full-screen",
                "Capture full screen",
                true,
                None::<&str>,
            )?;
            let open_lib =
                MenuItem::with_id(app, "tray:open-library", "Open library", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&capture_item, &open_lib, &quit])?;

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "tray:capture-full-screen" => {
                        let _ = app.emit("hotkey:capture-full-screen", ());
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

            info!("snapper-keeper started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!(error = %e, "tauri runtime exited");
        });
}
