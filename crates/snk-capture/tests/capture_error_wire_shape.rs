//! Wire-shape snapshot for `CaptureError`. CI fails on drift.
//!
//! See `crates/snk-library/tests/library_error_wire_shape.rs` for
//! background on why this contract is locked down.

use serde_json::json;
use snk_capture::CaptureError;
use snk_library::LibraryError;

#[test]
fn no_monitors_variant_wire_shape() {
    let e = CaptureError::NoMonitors;
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(actual, json!({"kind": "no-monitors"}));
}

#[test]
fn window_not_found_variant_wire_shape() {
    let e = CaptureError::WindowNotFound { id: 42 };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "window-not-found",
            "id": 42,
        })
    );
}

#[test]
fn os_variant_wire_shape() {
    let e = CaptureError::Os {
        message: "xcap: monitor handle invalid".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "os",
            "message": "xcap: monitor handle invalid",
        })
    );
}

#[test]
fn encode_variant_wire_shape() {
    let e = CaptureError::Encode {
        message: "png encode failed".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "encode",
            "message": "png encode failed",
        })
    );
}

// ============================================================
// CRITICAL: nested-tag flattening behavior
// ============================================================
//
// `CaptureError` uses `#[serde(tag = "kind")]`. Its `Library` variant
// wraps `LibraryError`, which ALSO uses `#[serde(tag = "kind")]`. When
// serde serializes the wrapper, the INNER enum's `kind` wins and the
// OUTER `"library"` discriminator is LOST.
//
// Concretely:
//   CaptureError::Library(LibraryError::NotFound { what: "x" })
// serializes as:
//   {"kind": "not-found", "what": "x"}
//
// NOT (as one might expect):
//   {"kind": "library", "what": "x"}
// or nested:
//   {"kind": "library", "library": {"kind": "not-found", "what": "x"}}
//
// This means the frontend cannot distinguish between an error that came
// through snk-capture's IPC vs snk-clipboard's vs snk-annotate's vs
// snk-library's directly — they all flatten to the inner LibraryError
// shape. Currently the frontend only routes on `kind` so this happens
// to work, but it's a latent design issue. Tracked separately for the
// project to decide whether to redesign the wrapping pattern (e.g.
// content + tag, or flat-prefixed kind like "library/not-found").
//
// These tests lock down the current behavior. Failing here means either:
//   (a) you intentionally changed the wrapping pattern — update both
//       the tests and the frontend routing code,
//   (b) you changed LibraryError's variants — the discriminator loss
//       means a LibraryError addition silently changes the wire shape
//       of every wrapper crate too.
// ============================================================

#[test]
fn library_wrapping_not_found_wire_shape() {
    let e = CaptureError::Library(LibraryError::NotFound {
        what: "capture xyz".into(),
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "not-found",
            "what": "capture xyz",
        })
    );
}

#[test]
fn library_wrapping_database_wire_shape() {
    let e = CaptureError::Library(LibraryError::Database {
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
fn library_wrapping_io_wire_shape() {
    let e = CaptureError::Library(LibraryError::Io {
        path: "/data/x.png".into(),
        reason: "permission denied".into(),
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "io",
            "path": "/data/x.png",
            "reason": "permission denied",
        })
    );
}
