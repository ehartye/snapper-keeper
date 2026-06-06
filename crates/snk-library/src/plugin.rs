use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Manager, Runtime};

use crate::Db;

pub struct LibraryState {
    pub db: Arc<Db>,
    pub root: PathBuf,
}

/// Payload emitted as `library:migration-failed` when a DB migration fails
/// and a backup has been restored.
#[derive(Serialize, Clone)]
struct MigrationFailedPayload {
    backup_path: String,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-library")
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_captures,
            crate::commands::get_capture,
            crate::commands::soft_delete_capture,
            crate::commands::hard_delete_capture,
            crate::commands::purge_trash,
            crate::commands::set_capture_pinned,
            crate::commands::list_clipboard_items,
            crate::commands::get_clipboard_item,
            crate::commands::toggle_clipboard_pin,
            crate::commands::search_library,
            crate::commands::list_tags,
            crate::commands::create_tag,
            crate::commands::update_tag,
            crate::commands::delete_tag,
            crate::commands::assign_tag,
            crate::commands::remove_tag,
            crate::commands::list_capture_tags,
            crate::commands::get_setting,
            crate::commands::set_setting,
            crate::commands::get_theme,
        ])
        .setup(|app, _api| {
            let root = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("resolve app data dir: {e}"))?;
            let db_path = root.join("snapper-keeper.db");

            let db = match Db::open(&db_path) {
                Ok(db) => db,
                Err(crate::LibraryError::Migration {
                    recoverable: true,
                    backup_path,
                    ..
                }) => {
                    // The backup has been restored; notify the frontend and
                    // open the DB at its prior (restored) schema version so
                    // the app can start and show the user an error modal.
                    let _ = app.emit(
                        "library:migration-failed",
                        MigrationFailedPayload {
                            backup_path: backup_path.unwrap_or_default(),
                        },
                    );
                    Db::open_no_migrate(&db_path)
                        .map_err(|e| format!("reopen db after migration restore: {e}"))?
                }
                Err(e) => return Err(format!("open db: {e}").into()),
            };

            app.manage(LibraryState {
                db: Arc::new(db),
                root,
            });
            Ok(())
        })
        .build()
}
