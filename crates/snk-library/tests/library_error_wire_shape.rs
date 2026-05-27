//! Wire-shape snapshot for `LibraryError`. CI fails on drift.
//!
//! `LibraryError` is wrapped by `CaptureError::Library(...)`,
//! `ClipboardError::Library(...)`, `AnnotateError::Library(...)`. The
//! frontend deserializes these wrapper errors and routes on
//! `error.kind`. Any change to the on-wire shape of `LibraryError`'s
//! variants — or the `serde(tag = "kind")` discriminator name itself —
//! is a frontend-breaking IPC contract change.
//!
//! These tests document the contract; failing here is intentional —
//! update the snapshot only when you've verified the frontend handles
//! the new shape.

use serde_json::json;
use snk_library::LibraryError;

#[test]
fn database_variant_wire_shape() {
    let e = LibraryError::Database {
        message: "boom".into(),
        retryable: false,
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "database",
            "message": "boom",
            "retryable": false,
        })
    );
}

#[test]
fn io_variant_wire_shape() {
    let e = LibraryError::Io {
        path: "/tmp/x.png".into(),
        reason: "permission denied (os error 13)".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "io",
            "path": "/tmp/x.png",
            "reason": "permission denied (os error 13)",
        })
    );
}

#[test]
fn migration_variant_wire_shape() {
    let e = LibraryError::Migration {
        from: 3,
        to: 4,
        recoverable: true,
        backup_path: None,
        detail: String::new(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "migration",
            "from": 3,
            "to": 4,
            "recoverable": true,
        })
    );
}

#[test]
fn not_found_variant_wire_shape() {
    let e = LibraryError::NotFound {
        what: "capture 019e5d59".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "not-found",
            "what": "capture 019e5d59",
        })
    );
}

#[test]
fn persist_variant_wire_shape() {
    let e = LibraryError::Persist {
        detail: "serialize words_json: invalid utf-8".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "persist",
            "detail": "serialize words_json: invalid utf-8",
        })
    );
}

#[test]
fn discriminator_uses_kebab_case() {
    // Sanity for the wrapper enums: they all use the same discriminator
    // name. If this ever changes, every wrapper-crate snapshot test in
    // the workspace would also fail.
    let e = LibraryError::NotFound { what: "x".into() };
    let actual = serde_json::to_string(&e).expect("serialize");
    assert!(actual.contains("\"kind\":\"not-found\""));
    assert!(!actual.contains("\"type\":"));
    assert!(!actual.contains("NotFound"));
}
