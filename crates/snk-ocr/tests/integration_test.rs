//! Integration test — requires tesseract installed on the machine.
//! Skips gracefully if tesseract is not available.

use std::path::Path;
use std::process::Command;

fn tesseract_available() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn sidecar_extracts_blank_image_without_error() {
    if !tesseract_available() {
        eprintln!("SKIP: tesseract not installed");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let img_path = dir.path().join("blank.png");

    // Blank white image — tesseract should process without error,
    // producing little or no text.
    let img = image::RgbaImage::from_pixel(200, 50, image::Rgba([255, 255, 255, 255]));
    img.save(&img_path).unwrap();

    let result = snk_ocr::sidecar::run_tesseract(&img_path, "eng");
    match result {
        Ok(output) => {
            assert!(
                output.text.len() < 10,
                "blank image should have minimal text"
            );
        }
        Err(e) => {
            assert!(
                e.contains("empty") || e.contains("Empty") || e.contains("exit"),
                "unexpected error: {e}"
            );
        }
    }
}

#[test]
fn sidecar_extracts_hello_world_from_fixture() {
    if !tesseract_available() {
        eprintln!("SKIP: tesseract not installed");
        return;
    }

    // 256x64 PNG with the literal text "hello world" rendered in Arial 32pt
    // bold — committed alongside the test rather than generated at runtime
    // so the test doesn't depend on which font-rendering crates are
    // available on the build machine.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("hello-world.png");
    assert!(fixture.is_file(), "missing fixture: {}", fixture.display());

    let output = snk_ocr::sidecar::run_tesseract(&fixture, "eng")
        .expect("hello-world fixture should OCR cleanly");

    let normalized = output.text.to_lowercase();
    assert!(
        normalized.contains("hello") && normalized.contains("world"),
        "expected extracted text to contain both 'hello' and 'world'; got: {:?}",
        output.text
    );
}

#[test]
fn sidecar_retries_on_bad_path() {
    if !tesseract_available() {
        eprintln!("SKIP: tesseract not installed");
        return;
    }

    let result = snk_ocr::sidecar::run_tesseract(Path::new("/nonexistent/image.png"), "eng");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("3 attempts"),
        "should report retry exhaustion: {err}"
    );
}
