//! Wire-shape snapshot for `ClipboardError`. CI fails on drift.
//!
//! See `crates/snk-library/tests/library_error_wire_shape.rs` for the
//! contract background. Wrapping pattern is adjacent-tagged
//! (`#[serde(tag = "kind", content = "data")]`) per #104.

use serde_json::json;
use snk_clipboard::ClipboardError;
use snk_library::LibraryError;

#[test]
fn library_wrapping_not_found_wire_shape() {
    // Adjacent-tag: outer wrapper identity ("library") preserved at
    // top level; inner LibraryError nested under "data".
    let e = ClipboardError::Library(LibraryError::NotFound {
        what: "item xyz".into(),
    });
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "library",
            "data": {
                "kind": "not-found",
                "what": "item xyz",
            },
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
fn access_variant_wire_shape() {
    let e = ClipboardError::Access {
        message: "OpenClipboard failed".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "access",
            "data": { "message": "OpenClipboard failed" },
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
            "data": { "reason": "target window doesn't accept text" },
        })
    );
}

#[test]
fn own_not_found_variant_wire_shape() {
    // Post-#104: ClipboardError::NotFound has its own discriminator
    // ("not-found") at the TOP level, while a library NotFound nests
    // its kind under data. The collision that existed pre-#104 is gone.
    let e = ClipboardError::NotFound {
        what: "item abc".into(),
    };
    let actual = serde_json::to_value(&e).expect("serialize");
    assert_eq!(
        actual,
        json!({
            "kind": "not-found",
            "data": { "what": "item abc" },
        })
    );
}

#[test]
fn own_not_found_does_not_collide_with_library_not_found() {
    // Post-#104: the adjacent-tag fix means own.NotFound and
    // Library(LibraryError::NotFound) are now DIFFERENT wire shapes.
    // This test inverts the pre-#104 `own_not_found_collides_with_library_not_found`
    // assertion that documented the bug. The fix is what makes this
    // distinct.
    let own = ClipboardError::NotFound { what: "x".into() };
    let wrapped = ClipboardError::Library(LibraryError::NotFound { what: "x".into() });
    let own_json = serde_json::to_value(&own).expect("serialize");
    let wrapped_json = serde_json::to_value(&wrapped).expect("serialize");
    assert_ne!(
        own_json, wrapped_json,
        "post-#104 fix: own and library variants must have distinct wire shapes"
    );
    // And explicitly verify the distinguishing key:
    assert_eq!(own_json["kind"], "not-found");
    assert_eq!(wrapped_json["kind"], "library");
}
