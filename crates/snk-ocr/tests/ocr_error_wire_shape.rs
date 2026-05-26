//! Documents the wire shape of OcrError variants. Failing here is intentional;
//! update the snapshot when the contract changes (frontend assumes specific kind values).

use serde_json::json;
use snk_ocr::OcrError;

#[test]
fn backend_unavailable_kebab_case() {
    let e = OcrError::BackendUnavailable { reason: "x".into() };
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({"kind": "backend-unavailable", "reason": "x"})
    );
}

#[test]
fn no_recognizer_language_kebab_case() {
    let e = OcrError::NoRecognizerLanguage { detail: "x".into() };
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({"kind": "no-recognizer-language", "detail": "x"})
    );
}

#[test]
fn recognize_kebab_case() {
    let e = OcrError::Recognize { detail: "x".into() };
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({"kind": "recognize", "detail": "x"})
    );
}

#[test]
fn image_load_kebab_case() {
    let e = OcrError::ImageLoad {
        path: "x".into(),
        detail: "y".into(),
    };
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({"kind": "image-load", "path": "x", "detail": "y"})
    );
}
