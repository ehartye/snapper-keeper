//! Integration tests for snk-ocr — exercises the active platform backend.
//! Vision on macOS, WinOcr on Windows. No subprocess.
//!
//! Gated to Mac+Windows at the file level: on Linux there's no platform
//! OcrBackend, so registering #[test] fns that would call a panic!()
//! make_backend() arm causes the Linux CI runner to fail. With the file
//! gate, the tests don't even compile on Linux and cargo skips them
//! cleanly (the binary still builds; it just has zero tests).
#![cfg(any(target_os = "macos", target_os = "windows"))]

use std::path::PathBuf;

use snk_ocr::backend::OcrBackend;

#[cfg(target_os = "macos")]
fn make_backend() -> Option<Box<dyn OcrBackend>> {
    Some(Box::new(
        snk_ocr::vision::VisionBackend::new().expect("Vision backend should construct"),
    ))
}

#[cfg(target_os = "windows")]
fn make_backend() -> Option<Box<dyn OcrBackend>> {
    match snk_ocr::winocr::WinOcrBackend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(e) => {
            eprintln!("WinOcrBackend unavailable on this machine; skipping: {e:?}");
            None
        }
    }
}

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p
}

#[test]
fn recognize_hello_world_returns_text_and_words() {
    let Some(b) = make_backend() else { return };
    let r = b.recognize(&fixture("hello-world.png")).expect("recognize");
    let text_lower = r.text.to_lowercase();
    assert!(
        text_lower.contains("hello"),
        "text should contain 'hello'; got {:?}",
        r.text
    );
    assert!(
        text_lower.contains("world"),
        "text should contain 'world'; got {:?}",
        r.text
    );
    assert!(!r.words.is_empty(), "should return at least one word");
    let first = &r.words[0];
    assert!(
        first.bbox.w > 0.0 && first.bbox.w <= 1.0,
        "bbox w should be normalized 0..1; got {}",
        first.bbox.w
    );
    assert!(
        first.bbox.h > 0.0 && first.bbox.h <= 1.0,
        "bbox h should be normalized 0..1; got {}",
        first.bbox.h
    );
}

#[test]
fn engine_version_is_populated() {
    let Some(b) = make_backend() else { return };
    let v = b.engine_version();
    assert!(!v.is_empty(), "engine_version should not be empty");
}
