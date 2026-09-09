use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-capture")
        .setup(|app, _api| {
            // Register the app with TCC so the SCK permission prompt appears
            // on first capture attempt rather than silently returning black frames.
            crate::permissions::request_screen_recording_access();
            app.manage(crate::commands::PreviewSessionState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::capture_full_screen,
            crate::commands::capture_window,
            crate::commands::capture_region,
            crate::commands::list_capturable_windows,
            crate::commands::grab_screen_preview,
            crate::commands::capture_cursor_position,
            crate::commands::capture_permission_status,
            crate::commands::open_screen_recording_settings,
        ])
        .build()
}
