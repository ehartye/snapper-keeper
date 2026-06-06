//! #54 — fire TS-binding-shaped JSON through the IPC mock for snk-clipboard
//! commands, locking argument naming and exercising the #49 authz wiring.
//!
//! Windows-gated: see the note in snk-library/tests/ipc_command_args.rs — the
//! mock runtime's webview layer can't launch in a non-interactive Windows
//! session. CI's `rust-test` job runs these on ubuntu-latest.
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
            snk_clipboard::commands::paste_item,
            snk_clipboard::commands::clipboard_status,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    app.manage(snk_library::LibraryState {
        db: Arc::new(db),
        root: tmp.path().to_path_buf(),
    });
    app.manage(snk_clipboard::health::ClipboardHealth::new());
    let webview = WebviewWindowBuilder::new(&app, label, Default::default())
        .build()
        .unwrap();
    (tmp, app, webview)
}

#[test]
fn clipboard_status_returns_available_shape() {
    let (_tmp, _app, wv) = build("clipboard-popup");
    let res = invoke(&wv, "clipboard_status", serde_json::json!({})).expect("status ok");
    // Locks the { available, last_error } shape the TS binding expects.
    assert_eq!(res["available"], true);
    assert!(res.get("last_error").is_some());
}

#[test]
fn paste_item_from_popup_parses_id_arg_then_hits_not_found() {
    // clipboard-popup is authorized for paste_item, so authz passes and the
    // command reaches clipboard::get, which returns NotFound for a bogus id —
    // proving the `id` arg deserialized (not an arg-name error) without
    // triggering a real paste.
    let (_tmp, _app, wv) = build("clipboard-popup");
    let err = invoke(
        &wv,
        "paste_item",
        serde_json::json!({ "id": "does-not-exist" }),
    )
    .expect_err("missing item must error");
    // ClipboardError::Library(LibraryError::NotFound) → kind "library".
    assert_eq!(
        err["kind"], "library",
        "expected a library not-found, got {err}"
    );
}

#[test]
fn paste_item_from_unauthorized_window_is_rejected() {
    // library is not in PASTE_ITEM_WINDOWS — rejected before any paste.
    let (_tmp, _app, wv) = build("library");
    let err = invoke(&wv, "paste_item", serde_json::json!({ "id": "x" }))
        .expect_err("unauthorized window must be rejected");
    assert_eq!(err["kind"], "unauthorized");
    assert_eq!(err["window"], "library");
    assert_eq!(err["command"], "paste_item");
}
