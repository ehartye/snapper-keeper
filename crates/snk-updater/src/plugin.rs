use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use snk_library::LibraryState;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::UpdaterExt;
use thiserror::Error;
use tracing::{error, info, warn};
use ts_rs::TS;

#[derive(Debug, Error, Serialize, TS)]
#[ts(
    export,
    export_to = "../../../packages/snk-updater/src/generated/errors.ts"
)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UpdaterError {
    #[error("updater init failed: {detail}")]
    Init { detail: String },
}

pub type Result<T> = std::result::Result<T, UpdaterError>;

const UPDATER_ENABLED_KEY: &str = "updater.enabled";
const UPDATER_HIGHEST_SEEN_KEY: &str = "updater.highest_seen_version";
const UPDATER_ALLOW_ROLLBACK_KEY: &str = "updater.allow_rollback";

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
    // `new()` is only used by the test suite — production code uses
    // `with_status(...)` so the initial status can reflect a policy
    // suppression at app startup. Gate to #[cfg(test)] so the
    // workspace-wide `cargo clippy -- -D warnings` (which doesn't pass
    // --all-targets) doesn't flag the function as dead code.
    #[cfg(test)]
    fn new() -> Self {
        Self::with_status(UpdateStatus::Idle)
    }

    fn with_status(status: UpdateStatus) -> Self {
        Self {
            status: Mutex::new(status),
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

fn updater_enabled(db: &snk_library::Db) -> bool {
    snk_library::settings::get(db, UPDATER_ENABLED_KEY)
        .ok()
        .flatten()
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

fn allow_rollback(db: &snk_library::Db) -> bool {
    snk_library::settings::get(db, UPDATER_ALLOW_ROLLBACK_KEY)
        .ok()
        .flatten()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn highest_seen_version(db: &snk_library::Db) -> Option<semver::Version> {
    snk_library::settings::get(db, UPDATER_HIGHEST_SEEN_KEY)
        .ok()
        .flatten()
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|s| semver::Version::parse(&s).ok())
}

fn store_highest_seen(db: &snk_library::Db, version: &str) {
    let _ = snk_library::settings::set(
        db,
        UPDATER_HIGHEST_SEEN_KEY,
        &serde_json::Value::String(version.to_string()),
    );
}

/// Outcome of the downgrade-floor check.
#[derive(Debug, PartialEq, Eq)]
enum DowngradeDecision {
    /// Proceed with the update; persist `new_highest` as the highest-seen version.
    Allow { new_highest: String },
    /// Refuse: the offered version is below the floor and rollback isn't allowed.
    Block,
}

/// Decide whether an offered update is an allowed upgrade or a blocked
/// downgrade. The floor is the greater of the highest-ever-seen version and
/// the currently-running version; an offer strictly below the floor is blocked
/// unless the user opted into rollback. Tauri's updater already refuses offers
/// at or below the *current* version, so this primarily defends against being
/// pinned to a stale-but-validly-signed release below one already seen.
fn evaluate_downgrade(
    offered: &semver::Version,
    highest_seen: Option<&semver::Version>,
    current: &semver::Version,
    allow_rollback: bool,
) -> DowngradeDecision {
    let floor = match highest_seen {
        Some(h) if h > current => h,
        _ => current,
    };
    if !allow_rollback && offered < floor {
        DowngradeDecision::Block
    } else {
        let new_highest = if offered > floor { offered } else { floor };
        DowngradeDecision::Allow {
            new_highest: new_highest.to_string(),
        }
    }
}

/// Whether an updater error is a signature-verification failure — terminal for
/// the process lifetime — versus a recoverable network/IO error. A bad
/// signature means the update channel is compromised or misconfigured, so we
/// stop trying rather than retrying against a hostile/broken endpoint.
fn is_signature_error(e: &tauri_plugin_updater::Error) -> bool {
    use tauri_plugin_updater::Error;
    matches!(
        e,
        Error::Minisign(_) | Error::SignatureUtf8(_) | Error::Base64(_)
    )
}

fn suppressed_by_policy_status<R: Runtime>(app: &AppHandle<R>) -> Option<UpdateStatus> {
    let lib = app.try_state::<LibraryState>()?;
    (!updater_enabled(&lib.db)).then_some(UpdateStatus::SuppressedByPolicy {
        reason: SuppressionReason::UserDisabled,
    })
}

fn set_and_emit_status<R: Runtime>(app: &AppHandle<R>, status: UpdateStatus) {
    app.state::<UpdaterState>().set_status(status.clone());
    let _ = app.emit("updater:status-changed", status);
}

#[tauri::command]
pub async fn check_for_update<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus> {
    do_update_check(app).await
}

#[tauri::command]
pub fn get_update_status<R: Runtime>(app: AppHandle<R>) -> UpdateStatus {
    if let Some(status) = suppressed_by_policy_status(&app) {
        app.state::<UpdaterState>().set_status(status.clone());
        return status;
    }

    let status = app.state::<UpdaterState>().get_status();
    match status {
        UpdateStatus::SuppressedByPolicy {
            reason: SuppressionReason::UserDisabled,
        } => UpdateStatus::Idle,
        other => other,
    }
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

async fn do_update_check<R: Runtime>(app: AppHandle<R>) -> Result<UpdateStatus> {
    if let Some(status) = suppressed_by_policy_status(&app) {
        set_and_emit_status(&app, status.clone());
        return Ok(status);
    }

    let state = app.state::<UpdaterState>();
    state.set_status(UpdateStatus::Checking);
    state.set_last_check_at(chrono::Utc::now().timestamp_millis());
    let _ = app.emit("updater:status-changed", UpdateStatus::Checking);

    let updater = app.updater().map_err(|e| UpdaterError::Init {
        detail: e.to_string(),
    })?;

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            info!(%version, "update available");

            // Downgrade floor: refuse an offer below the highest-ever-seen
            // version (or the running version) unless the user enabled
            // rollback. Persist the new high-water mark when we accept.
            if let Some(lib) = app.try_state::<LibraryState>() {
                match semver::Version::parse(&version) {
                    Ok(offered) => {
                        let current = app.package_info().version.clone();
                        let seen = highest_seen_version(&lib.db);
                        match evaluate_downgrade(
                            &offered,
                            seen.as_ref(),
                            &current,
                            allow_rollback(&lib.db),
                        ) {
                            DowngradeDecision::Block => {
                                warn!(
                                    %version,
                                    "update blocked: version is below the downgrade floor \
                                     (enable updater.allow_rollback to override)"
                                );
                                let status = UpdateStatus::SuppressedByPolicy {
                                    reason: SuppressionReason::DowngradeBlocked,
                                };
                                set_and_emit_status(&app, status.clone());
                                return Ok(status);
                            }
                            DowngradeDecision::Allow { new_highest } => {
                                store_highest_seen(&lib.db, &new_highest);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            %version, error = %e,
                            "offered version is not valid semver; skipping downgrade-floor check"
                        );
                    }
                }
            }

            let status = UpdateStatus::Available {
                version: version.clone(),
                urgency: Urgency::Normal,
            };
            set_and_emit_status(&app, status.clone());

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
                        // A signature-verification failure is terminal: the
                        // update channel is compromised or misconfigured, so
                        // RejectedBySignature (a one-way state) disables the
                        // updater for the rest of the process. Network/IO
                        // errors stay retryable.
                        let status = if is_signature_error(&e) {
                            error!(
                                error = %e,
                                "update signature verification FAILED — updater disabled for this process"
                            );
                            UpdateStatus::RejectedBySignature
                        } else {
                            error!(error = %e, "update download failed");
                            UpdateStatus::Error {
                                reason: UpdateErrorKind::DownloadFailed,
                                retryable: true,
                            }
                        };
                        err_handle
                            .state::<UpdaterState>()
                            .set_status(status.clone());
                        let _ = err_handle.emit("updater:status-changed", status);
                    }
                }
            });

            Ok(status)
        }
        Ok(None) => {
            info!("no update available");
            set_and_emit_status(&app, UpdateStatus::Idle);
            Ok(UpdateStatus::Idle)
        }
        Err(e) => {
            warn!(error = %e, "update check failed");
            let status = UpdateStatus::Error {
                reason: UpdateErrorKind::CheckFailed,
                retryable: true,
            };
            set_and_emit_status(&app, status.clone());
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
            let initial_status =
                suppressed_by_policy_status(app.app_handle()).unwrap_or(UpdateStatus::Idle);
            app.manage(UpdaterState::with_status(initial_status));

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
    fn set_policy_suppressed_status() {
        let state = UpdaterState::new();
        state.set_status(UpdateStatus::SuppressedByPolicy {
            reason: SuppressionReason::UserDisabled,
        });
        assert_eq!(
            state.get_status(),
            UpdateStatus::SuppressedByPolicy {
                reason: SuppressionReason::UserDisabled
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
    fn serde_roundtrip_policy_suppression_variant() {
        let suppressed = UpdateStatus::SuppressedByPolicy {
            reason: SuppressionReason::UserDisabled,
        };
        let json = serde_json::to_string(&suppressed).unwrap();
        assert!(json.contains("\"kind\":\"suppressed-by-policy\""));
        assert!(json.contains("\"reason\":\"user-disabled\""));
        let parsed: UpdateStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, suppressed);
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

    #[test]
    fn updater_enabled_defaults_true() {
        let temp = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&temp.path().join("snapper-keeper.db")).unwrap();
        assert!(updater_enabled(&db));
    }

    #[test]
    fn updater_enabled_reads_false_setting() {
        let temp = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&temp.path().join("snapper-keeper.db")).unwrap();
        snk_library::settings::set(&db, UPDATER_ENABLED_KEY, &serde_json::Value::Bool(false))
            .unwrap();
        assert!(!updater_enabled(&db));
    }

    fn v(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    #[test]
    fn downgrade_allows_a_normal_upgrade_and_advances_high_water_mark() {
        let d = evaluate_downgrade(&v("1.3.0"), Some(&v("1.2.0")), &v("1.2.0"), false);
        assert_eq!(
            d,
            DowngradeDecision::Allow {
                new_highest: "1.3.0".into()
            }
        );
    }

    #[test]
    fn downgrade_blocks_below_highest_seen_without_rollback() {
        // Saw 1.5.0, running 1.0.0, offered 1.3.0 → below the 1.5.0 floor.
        let d = evaluate_downgrade(&v("1.3.0"), Some(&v("1.5.0")), &v("1.0.0"), false);
        assert_eq!(d, DowngradeDecision::Block);
    }

    #[test]
    fn downgrade_permitted_when_rollback_enabled() {
        let d = evaluate_downgrade(&v("1.3.0"), Some(&v("1.5.0")), &v("1.0.0"), true);
        // Rollback keeps the existing high-water mark rather than lowering it.
        assert_eq!(
            d,
            DowngradeDecision::Allow {
                new_highest: "1.5.0".into()
            }
        );
    }

    #[test]
    fn downgrade_floor_falls_back_to_current_when_no_history() {
        // Offer equal to current is not below the floor → allowed.
        let d = evaluate_downgrade(&v("2.0.0"), None, &v("2.0.0"), false);
        assert_eq!(
            d,
            DowngradeDecision::Allow {
                new_highest: "2.0.0".into()
            }
        );
        // Offer below current with no history is blocked.
        let d = evaluate_downgrade(&v("1.9.0"), None, &v("2.0.0"), false);
        assert_eq!(d, DowngradeDecision::Block);
    }

    #[test]
    fn signature_errors_are_terminal_network_errors_are_not() {
        use tauri_plugin_updater::Error;
        assert!(is_signature_error(&Error::SignatureUtf8("bad".into())));
        assert!(!is_signature_error(&Error::Network("timeout".into())));
        assert!(!is_signature_error(&Error::ReleaseNotFound));
    }
}
