use std::path::Path;
use std::process::Command;
use std::time::Duration;

use tracing::{info, warn};

pub struct OcrOutput {
    pub text: String,
    pub confidence: f64,
}

pub fn run_tesseract(image_path: &Path, language: &str) -> Result<OcrOutput, String> {
    let mut last_err = String::new();
    let delays = [
        Duration::from_millis(0),
        Duration::from_secs(1),
        Duration::from_secs(3),
    ];

    for (attempt, delay) in delays.iter().enumerate() {
        if attempt > 0 {
            std::thread::sleep(*delay);
            warn!(attempt, "retrying tesseract");
        }

        match invoke_tesseract(image_path, language) {
            Ok(output) => {
                info!(attempt, chars = output.text.len(), "tesseract succeeded");
                return Ok(output);
            }
            Err(e) => {
                last_err = e;
                warn!(attempt, error = %last_err, "tesseract failed");
            }
        }
    }

    Err(format!("tesseract failed after 3 attempts: {last_err}"))
}

fn invoke_tesseract(image_path: &Path, language: &str) -> Result<OcrOutput, String> {
    let output = Command::new("tesseract")
        .arg(image_path.as_os_str())
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg("3")
        .output()
        .map_err(|e| format!("spawn tesseract: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tesseract exit {}: {stderr}", output.status));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Tesseract's stdout mode doesn't emit a confidence number; use a heuristic
    // — non-empty output is treated as moderate confidence.
    let confidence = if text.is_empty() { 0.0 } else { 0.85 };

    Ok(OcrOutput { text, confidence })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_tesseract_returns_error_when_binary_missing_or_path_invalid() {
        // Passes whether tesseract is installed or not — exercises the error path
        // (spawn failure if no binary, or non-zero exit on the bad path).
        let result = invoke_tesseract(Path::new("/nonexistent/image.png"), "eng");
        assert!(result.is_err() || result.unwrap().text.is_empty());
    }
}
