//! Wire-shape snapshot for `AnnotateError`. CI fails on drift.
//!
//! See `crates/snk-library/tests/library_error_wire_shape.rs` for the
//! contract background and `crates/snk-capture/tests/capture_error_wire_shape.rs`
//! for the critical-comment about the nested-tag flattening behavior.

use serde_json::json;
use snk_annotate::AnnotateError;
use snk_library::LibraryError;

#[test]
fn image_variant_wire_shape() {
    let e = AnnotateError::Image {
        message: "PNG decoder failed at offset 42".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "image",
            "message": "PNG decoder failed at offset 42",
        })
    );
}

#[test]
fn invalid_input_variant_wire_shape() {
    let e = AnnotateError::InvalidInput {
        reason: "state_json missing required field 'shapes'".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "invalid-input",
            "reason": "state_json missing required field 'shapes'",
        })
    );
}

#[test]
fn library_wrapping_not_found_wire_shape() {
    // Inner-wins flattening (see capture_error_wire_shape.rs for full
    // explanation). The outer "library" discriminator is LOST.
    let e = AnnotateError::Library(LibraryError::NotFound {
        what: "capture for annotation".into(),
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "not-found",
            "what": "capture for annotation",
        })
    );
}

#[test]
fn library_wrapping_io_wire_shape() {
    let e = AnnotateError::Library(LibraryError::Io {
        path: "/data/annotations/x.json".into(),
        reason: "permission denied".into(),
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "io",
            "path": "/data/annotations/x.json",
            "reason": "permission denied",
        })
    );
}
