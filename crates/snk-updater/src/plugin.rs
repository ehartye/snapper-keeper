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
    Available {
        version: String,
        urgency: Urgency,
    },
    Downloading {
        progress: f32,
    },
    Ready {
        version: String,
    },
    Installing,
    Error {
        reason: UpdateErrorKind,
        retryable: bool,
    },
    RejectedBySignature,
    SuppressedByPolicy {
        reason: SuppressionReason,
    },
    Skipped {
        until_epoch_ms: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Urgency {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateErrorKind {
    CheckFailed,
    DownloadFailed,
    InstallFailed,
    SignatureVerificationFailed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SuppressionReason {
    UserDisabled,
    StoreEdition,
    DowngradeBlocked,
    UnknownPolicy,
}

pub struct UpdaterState {
    status: Mutex<UpdateStatus>,
    last_check_at: Mutex<Option<i64>>,
}

impl UpdaterState {
    fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::Idle),
            last_check_at: Mutex::new(None),
        }
    }

    fn set_status(&self, s: UpdateStatus) {
        if let Ok(mut lock) = self.status.lock() {
            if matches!(&*lock, UpdateStatus::RejectedBySignature)
                && !matches!(&s, UpdateStatus::RejectedBySignature)
            {
                return;
            }
            *lock = s;
        }
    }

    fn get_status(&self) -> UpdateStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(UpdateStatus::Idle)
    }

    fn set_last_check_at(&self, ts: i64) {
        if let Ok(mut lock) = self.last_check_at.lock() {
            *lock = Some(ts);
        }
    }

    fn get_last_check_at(&self) -> Option<i64> {
        self.last_check_at.lock().ok().and_then(|l| *l)
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

#[tauri::command]
pub fn get_last_check_at<R: Runtime>(app: AppHandle<R>) -> Option<i64> {
    app.state::<UpdaterState>().get_last_check_at()
}

#[tauri::command]
pub fn restart_app<R: Runtime>(app: AppHandle<R>) {
    let status = UpdateStatus::Installing;
    app.state::<UpdaterState>().set_status(status.clone());
    let _ = app.emit("updater:status-changed", status);
    app.restart();
}

async fn do_update_check<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus, String> {
    let state = app.state::<UpdaterState>();
    state.set_status(UpdateStatus::Checking);
    state.set_last_check_at(chrono::Utc::now().timestamp_millis());
    let _ = app.emit("updater:status-changed", UpdateStatus::Checking);

    let updater = app.updater().map_err(|e| format!("updater init: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            info!(%version, "update available");
            let status = UpdateStatus::Available {
                version: version.clone(),
                urgency: Urgency::Normal,
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
                            let progress = content_length
                                .map(|cl| (downloaded as f32 / cl as f32) * 100.0)
                                .unwrap_or(0.0);
                            let status = UpdateStatus::Downloading { progress };
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
                            reason: UpdateErrorKind::DownloadFailed,
                            retryable: true,
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
                reason: UpdateErrorKind::CheckFailed,
                retryable: true,
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
            get_update_status,
            get_last_check_at,
            restart_app
        ])
        .setup(|app, _api| {
            app.manage(UpdaterState::new());

            let handle = app.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                // Initial check ~5s after launch, then once every 24h.
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Err(e) = do_update_check(handle.clone()).await {
                    warn!(error = %e, "startup update check failed");
                }

                // `Delay` (vs default `Burst`) means a stretch of suspend
                // / sleep / sluggish runtime won't replay multiple missed
                // ticks in rapid succession when the runtime resumes.
                let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                // First tick fires immediately; the startup check above
                // already covered it, so consume it and skip.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_status_is_idle() {
        let state = UpdaterState::new();
        assert_eq!(state.get_status(), UpdateStatus::Idle);
    }

    #[test]
    fn set_and_get_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Checking);
        assert_eq!(state.get_status(), UpdateStatus::Checking);
    }

    #[test]
    fn set_available_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Available {
            version: "1.2.3".to_string(),
            urgency: Urgency::Normal,
        });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Available {
                version: "1.2.3".to_string(),
                urgency: Urgency::Normal
            }
        );
    }

    #[test]
    fn set_downloading_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Downloading { progress: 42.5 });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Downloading { progress: 42.5 }
        );
    }

    #[test]
    fn set_error_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::Error {
            reason: UpdateErrorKind::CheckFailed,
            retryable: true,
        });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Error {
                reason: UpdateErrorKind::CheckFailed,
                retryable: true
            }
        );
    }

    #[test]
    fn status_transitions() {
        let state = UpdaterState::new();
        assert_eq!(state.get_status(), UpdateStatus::Idle);

        state.set_status(UpdateStatus::Checking);
        assert_eq!(state.get_status(), UpdateStatus::Checking);

        state.set_status(UpdateStatus::Available {
            version: "2.0.0".to_string(),
            urgency: Urgency::Normal,
        });
        state.set_status(UpdateStatus::Downloading { progress: 50.0 });
        state.set_status(UpdateStatus::Ready {
            version: "2.0.0".to_string(),
        });
        assert_eq!(
            state.get_status(),
            UpdateStatus::Ready {
                version: "2.0.0".to_string()
            }
        );
    }

    #[test]
    fn serde_roundtrip_unit_variants() {
        let idle = UpdateStatus::Idle;
        let json = serde_json::to_string(&idle).unwrap();
        assert!(json.contains("\"kind\":\"idle\""));
        let parsed: UpdateStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, UpdateStatus::Idle);
    }

    #[test]
    fn serde_roundtrip_data_variants() {
        let available = UpdateStatus::Available {
            version: "3.0.0".to_string(),
            urgency: Urgency::High,
        };
        let json = serde_json::to_string(&available).unwrap();
        assert!(json.contains("\"kind\":\"available\""));
        assert!(json.contains("\"version\":\"3.0.0\""));
        assert!(json.contains("\"urgency\":\"high\""));
        let parsed: UpdateStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, available);
    }

    #[test]
    fn rejected_by_signature_is_terminal_for_process_lifetime() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::RejectedBySignature);
        state.set_status(UpdateStatus::Idle);
        assert_eq!(state.get_status(), UpdateStatus::RejectedBySignature);
    }

    #[test]
    fn updater_state_last_check_at_starts_none() {
        let state = UpdaterState::new();
        assert!(state.get_last_check_at().is_none());
    }

    #[test]
    fn updater_state_records_last_check_at() {
        let state = UpdaterState::new();
        state.set_last_check_at(1716662400000);
        assert_eq!(state.get_last_check_at(), Some(1716662400000));
    }
}
