use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-annotate")
        .invoke_handler(tauri::generate_handler![crate::commands::save_annotation])
        .build()
}
