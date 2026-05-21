use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-capture")
        .invoke_handler(tauri::generate_handler![
            crate::commands::capture_full_screen
        ])
        .build()
}
