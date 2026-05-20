use std::path::PathBuf;
use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

use crate::Db;

pub struct LibraryState {
    pub db: Arc<Db>,
    pub root: PathBuf,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-library")
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_captures,
            crate::commands::get_capture,
        ])
        .setup(|app, _api| {
            let root = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("resolve app data dir: {e}"))?;
            let db_path = root.join("snapper-keeper.db");
            let db = Db::open(&db_path).map_err(|e| format!("open db: {e}"))?;
            app.manage(LibraryState { db: Arc::new(db), root });
            Ok(())
        })
        .build()
}
