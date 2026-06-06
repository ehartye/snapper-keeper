//! #54 — fire TS-binding-shaped JSON through the IPC mock to lock command
//! argument naming on the Rust side. A rename of a Rust command parameter that
//! diverged from what the TS bindings send would fail deserialization here.
//!
//! These also exercise the #49 per-window authorization wiring end-to-end:
//! `set_setting` from an unauthorized window must be rejected even with
//! well-formed args.
//!
//! Windows-gated: `tauri::test`'s mock runtime instantiates the webview layer,
//! which fails to launch in a non-interactive Windows/WebView2 session
//! (STATUS_ENTRYPOINT_NOT_FOUND — the same constraint CLAUDE.md notes for
//! Windows runtime tests). CI's `rust-test` job runs on ubuntu-latest, where
//! these execute normally.
#![cfg(not(target_os = "windows"))]

use std::sync::Arc;

use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewWindow, WebviewWindowBuilder};

fn ipc_url() -> tauri::Url {
    if cfg!(any(windows, target_os = "android")) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    }
    .parse()
    .unwrap()
}

fn invoke(
    webview: &WebviewWindow<MockRuntime>,
    cmd: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: ipc_url(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<serde_json::Value>().unwrap())
}

/// Build a mock app exposing the library setting commands, backed by a fresh
/// temp DB, with a webview carrying `label` (so the authz middleware sees the
/// caller window). The TempDir is returned so the DB file outlives the app.
fn build(
    label: &str,
) -> (
    tempfile::TempDir,
    tauri::App<MockRuntime>,
    WebviewWindow<MockRuntime>,
) {
    let tmp = tempfile::tempdir().unwrap();
    let db = snk_library::Db::open(&tmp.path().join("sk.db")).unwrap();
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            snk_library::commands::get_setting,
            snk_library::commands::set_setting,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    app.manage(snk_library::LibraryState {
        db: Arc::new(db),
        root: tmp.path().to_path_buf(),
    });
    let webview = WebviewWindowBuilder::new(&app, label, Default::default())
        .build()
        .unwrap();
    (tmp, app, webview)
}

#[test]
fn get_setting_accepts_ts_key_arg() {
    let (_tmp, _app, wv) = build("library");
    // TS sends `{ key }`. A rename of the Rust param would fail to deserialize.
    let res = invoke(
        &wv,
        "get_setting",
        serde_json::json!({ "key": "nonexistent" }),
    );
    assert_eq!(res, Ok(serde_json::Value::Null));
}

#[test]
fn set_setting_from_authorized_window_accepts_ts_key_value_args() {
    let (_tmp, _app, wv) = build("settings");
    let res = invoke(
        &wv,
        "set_setting",
        serde_json::json!({ "key": "updater.enabled", "value": false }),
    );
    assert_eq!(res, Ok(serde_json::Value::Null));
}

#[test]
fn set_setting_from_unauthorized_window_is_rejected() {
    // clipboard-popup is not in SET_SETTING_WINDOWS — the #49 middleware must
    // reject this even though the args are well-formed.
    let (_tmp, _app, wv) = build("clipboard-popup");
    let err = invoke(
        &wv,
        "set_setting",
        serde_json::json!({ "key": "updater.enabled", "value": false }),
    )
    .expect_err("unauthorized window must be rejected");
    assert_eq!(err["kind"], "unauthorized");
    assert_eq!(err["window"], "clipboard-popup");
    assert_eq!(err["command"], "set_setting");
}
