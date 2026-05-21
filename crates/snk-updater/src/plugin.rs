use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::UpdaterExt;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available { version: String },
    Downloading { percent: f32 },
    Ready { version: String },
    Error { detail: String },
}

pub struct UpdaterState {
    status: Mutex<UpdateStatus>,
}

impl UpdaterState {
    fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::Idle),
        }
    }

    fn set_status(&self, s: UpdateStatus) {
        if let Ok(mut lock) = self.status.lock() {
            *lock = s;
        }
    }

    fn get_status(&self) -> UpdateStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(UpdateStatus::Idle)
    }
}

#[tauri::command]
pub async fn check_for_update<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus, String> {
    do_update_check(app).await
}

#[tauri::command]
pub fn get_update_status<R: Runtime>(app: AppHandle<R>) -> UpdateStatus {
    app.state::<UpdaterState>().get_status()
}

async fn do_update_check<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus, String> {
    let state = app.state::<UpdaterState>();
    state.set_status(UpdateStatus::Checking);
    let _ = app.emit("updater:status-changed", UpdateStatus::Checking);

    let updater = app.updater().map_err(|e| format!("updater init: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            info!(%version, "update available");
            let status = UpdateStatus::Available {
                version: version.clone(),
            };
            state.set_status(status.clone());
            let _ = app.emit("updater:status-changed", status.clone());

            let dl_handle = app.app_handle().clone();
            let done_handle = app.app_handle().clone();
            let err_handle = app.app_handle().clone();
            tokio::spawn(async move {
                let mut downloaded: u64 = 0;
                match update
                    .download_and_install(
                        |chunk, content_length| {
                            downloaded += chunk as u64;
                            let percent = content_length
                                .map(|cl| (downloaded as f32 / cl as f32) * 100.0)
                                .unwrap_or(0.0);
                            let status = UpdateStatus::Downloading { percent };
                            dl_handle.state::<UpdaterState>().set_status(status.clone());
                            let _ = dl_handle.emit("updater:status-changed", status);
                        },
                        || {
                            let status = UpdateStatus::Ready {
                                version: version.clone(),
                            };
                            done_handle
                                .state::<UpdaterState>()
                                .set_status(status.clone());
                            let _ = done_handle.emit("updater:status-changed", status);
                            info!(%version, "update ready — restart to apply");
                        },
                    )
                    .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        let status = UpdateStatus::Error {
                            detail: e.to_string(),
                        };
                        err_handle
                            .state::<UpdaterState>()
                            .set_status(status.clone());
                        let _ = err_handle.emit("updater:status-changed", status);
                        error!(error = %e, "update download failed");
                    }
                }
            });

            Ok(status)
        }
        Ok(None) => {
            info!("no update available");
            state.set_status(UpdateStatus::Idle);
            let _ = app.emit("updater:status-changed", UpdateStatus::Idle);
            Ok(UpdateStatus::Idle)
        }
        Err(e) => {
            warn!(error = %e, "update check failed");
            let status = UpdateStatus::Error {
                detail: e.to_string(),
            };
            state.set_status(status.clone());
            let _ = app.emit("updater:status-changed", status.clone());
            Ok(status)
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-updater")
        .invoke_handler(tauri::generate_handler![
            check_for_update,
            get_update_status
        ])
        .setup(|app, _api| {
            app.manage(UpdaterState::new());

            let handle = app.app_handle().clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Err(e) = do_update_check(handle.clone()).await {
                    warn!(error = %e, "startup update check failed");
                }

                let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
                interval.tick().await;
                loop {
                    interval.tick().await;
                    if let Err(e) = do_update_check(handle.clone()).await {
                        warn!(error = %e, "periodic update check failed");
                    }
                }
            });

            Ok(())
        })
        .build()
}
