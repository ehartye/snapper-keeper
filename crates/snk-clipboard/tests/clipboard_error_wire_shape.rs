//! Wire-shape snapshot for `ClipboardError`. CI fails on drift.
//!
//! See `crates/snk-library/tests/library_error_wire_shape.rs` for the
//! contract background and `crates/snk-capture/tests/capture_error_wire_shape.rs`
//! for the critical-comment about the nested-tag flattening behavior.

use serde_json::json;
use snk_clipboard::ClipboardError;
use snk_library::LibraryError;

#[test]
fn library_wrapping_not_found_wire_shape() {
    // Inner-wins flattening (see capture_error_wire_shape.rs for full
    // explanation). The outer "library" discriminator is LOST.
    let e = ClipboardError::Library(LibraryError::NotFound {
        what: "item xyz".into(),
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "not-found",
            "what": "item xyz",
        })
    );
}

#[test]
fn library_wrapping_database_wire_shape() {
    let e = ClipboardError::Library(LibraryError::Database {
        message: "table missing".into(),
        retryable: false,
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "database",
            "message": "table missing",
            "retryable": false,
        })
    );
}

#[test]
fn access_variant_wire_shape() {
    let e = ClipboardError::Access {
        message: "OpenClipboard failed".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "access",
            "message": "OpenClipboard failed",
        })
    );
}

#[test]
fn paste_failed_variant_wire_shape() {
    let e = ClipboardError::PasteFailed {
        reason: "target window doesn't accept text".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "paste-failed",
            "reason": "target window doesn't accept text",
        })
    );
}

#[test]
fn own_not_found_variant_wire_shape() {
    // CRITICAL: ClipboardError has its OWN `NotFound` variant. Combined
    // with the inner-wins flattening, ClipboardError::NotFound and
    // ClipboardError::Library(LibraryError::NotFound{...}) serialize as
    // IDENTICAL wire shapes — the frontend cannot distinguish "the
    // clipboard plugin says X" from "the library reported X via the
    // clipboard plugin's wrapping." See the cross-cutting issue in the
    // PR description for #56.
    let e = ClipboardError::NotFound {
        what: "item abc".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "not-found",
            "what": "item abc",
        })
    );
}

#[test]
fn own_not_found_collides_with_library_not_found() {
    // Explicit assertion of the collision documented above. If this
    // ever stops being true (i.e. the wrapping design is fixed) — the
    // assertion fails, which is the signal to update all snapshot
    // tests in the workspace.
    let own = ClipboardError::NotFound { what: "x".into() };
    let wrapped = ClipboardError::Library(LibraryError::NotFound { what: "x".into() });
    let own_json = serde_json::to_value(&own).expect("serialize");
    let wrapped_json = serde_json::to_value(&wrapped).expect("serialize");
    assert_eq!(
        own_json, wrapped_json,
        "if this fails the wrapping design has been fixed — update tests"
    );
}
