//! Wire-shape snapshot for `CaptureError`. CI fails on drift.
//!
//! See `crates/snk-library/tests/library_error_wire_shape.rs` for
//! background on why this contract is locked down.
//!
//! ## Wrapping pattern: adjacent tag
//!
//! `CaptureError` uses `#[serde(tag = "kind", content = "data")]`
//! (adjacent-tagged enum). Variant data — whether struct fields or a
//! wrapped enum — lives under the `data` key. The outer `kind` ALWAYS
//! reflects the wrapper variant, so the `Library(LibraryError)` case
//! correctly produces `{"kind": "library", "data": {"kind": "<inner>", ...}}`.
//!
//! This was changed in #104 (was previously `#[serde(tag = "kind")]`
//! only, which flattened the inner enum and lost the wrapper
//! identity). Frontend error-routing relies on the top-level `kind`;
//! when handling `library` errors specifically, route on
//! `error.data.kind` for the inner LibraryError variant.

use serde_json::json;
use snk_capture::CaptureError;
use snk_library::LibraryError;

#[test]
fn no_monitors_variant_wire_shape() {
    // Unit variants have no data — no `data` key in the output.
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
            "data": { "id": 42 },
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
            "data": { "message": "xcap: monitor handle invalid" },
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
            "data": { "message": "png encode failed" },
        })
    );
}

#[test]
fn library_wrapping_not_found_wire_shape() {
    // The wrapper's "library" discriminator is preserved at the top
    // level; the inner LibraryError appears nested under `data`. Compare
    // to the pre-#104 behavior where this produced `{"kind":"not-found",...}`
    // (inner-wins flattening lost the wrapper identity).
    let e = CaptureError::Library(LibraryError::NotFound {
        what: "capture xyz".into(),
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "library",
            "data": {
                "kind": "not-found",
                "what": "capture xyz",
            },
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
            "kind": "library",
            "data": {
                "kind": "database",
                "message": "table missing",
                "retryable": false,
            },
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
            "kind": "library",
            "data": {
                "kind": "io",
                "path": "/data/x.png",
                "reason": "permission denied",
            },
        })
    );
}
