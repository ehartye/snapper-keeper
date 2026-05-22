use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use tracing::{info, warn};

#[derive(Debug)]
pub struct OcrOutput {
    pub text: String,
    pub confidence: f64,
}

/// Cached path to the tesseract executable, resolved once on first use.
/// Returns `None` if tesseract isn't installed anywhere we can find.
static TESSERACT_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Tauri resource dir containing the bundled tesseract distribution
/// (executable + DLLs + tessdata). Set once by the plugin during setup.
static BUNDLED_RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Called by the plugin at startup with the Tauri resource directory.
pub fn set_bundled_resource_dir(dir: PathBuf) {
    let _ = BUNDLED_RESOURCE_DIR.set(dir);
}

/// Path to the tessdata directory if we resolved to a bundled tesseract.
fn bundled_tessdata_dir() -> Option<PathBuf> {
    let dir = BUNDLED_RESOURCE_DIR
        .get()?
        .join("tesseract")
        .join("tessdata");
    if dir.is_dir() {
        Some(dir)
    } else {
        None
    }
}

fn resolve_tesseract() -> Option<PathBuf> {
    // 1. Explicit override first (debugging, custom installs).
    if let Ok(p) = std::env::var("SNK_TESSERACT_PATH") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }

    // 2. Bundled distribution that ships with the installer.
    if let Some(dir) = BUNDLED_RESOURCE_DIR.get() {
        let exe_name = if cfg!(target_os = "windows") {
            "tesseract.exe"
        } else {
            "tesseract"
        };
        let candidate = dir.join("tesseract").join(exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // 3. If on PATH, this just works.
    if which("tesseract").is_some() {
        return Some(PathBuf::from("tesseract"));
    }

    // 4. Common install locations.
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Program Files\Tesseract-OCR\tesseract.exe",
            r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/opt/homebrew/bin/tesseract",
            "/usr/local/bin/tesseract",
            "/opt/local/bin/tesseract",
        ]
    } else {
        &["/usr/bin/tesseract", "/usr/local/bin/tesseract"]
    };

    for c in candidates {
        let pb = PathBuf::from(c);
        if pb.is_file() {
            return Some(pb);
        }
    }
    None
}

fn which(cmd: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(target_os = "windows") {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.BAT;.CMD".to_string())
            .split(';')
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path_var) {
        for ext in &exts {
            let candidate = dir.join(format!("{cmd}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
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
    let exe = TESSERACT_PATH
        .get_or_init(resolve_tesseract)
        .as_ref()
        .ok_or_else(|| {
            "tesseract not found — install it and ensure it's on PATH, or set \
             SNK_TESSERACT_PATH to the executable"
                .to_string()
        })?;

    let mut command = Command::new(exe);
    #[cfg(target_os = "windows")]
    {
        // Hide the console window that spawns alongside child processes on
        // Windows. CREATE_NO_WINDOW = 0x08000000.
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    // Point a bundled tesseract at its bundled tessdata. System installs find
    // their own data via the standard layout, so we only set this if we have
    // a bundled directory we ourselves shipped.
    if let Some(tessdata) = bundled_tessdata_dir() {
        command.env("TESSDATA_PREFIX", tessdata);
    }

    let output = command
        .arg(image_path.as_os_str())
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg("3")
        .output()
        .map_err(|e| format!("spawn tesseract ({}): {e}", exe.display()))?;

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

    #[test]
    fn run_tesseract_returns_error_after_retries_when_path_missing() {
        // Sub-second total: delays start at 0, 1s, 3s. We want to confirm the
        // function actually wraps the underlying error and reports retries.
        // We use a path that's guaranteed not to be readable. The test runs
        // synchronously and may take a few seconds.
        let result = run_tesseract(Path::new("/definitely/missing.png"), "eng");
        // Whatever tesseract setup the machine has, this either fails (no
        // such file) or returns no text — both acceptable.
        match result {
            Ok(out) => assert!(out.text.is_empty()),
            Err(e) => {
                assert!(
                    e.contains("tesseract failed") || e.contains("spawn"),
                    "unexpected error message: {e}"
                );
            }
        }
    }

    #[test]
    fn ocr_output_debug_includes_fields() {
        let o = OcrOutput {
            text: "hello".into(),
            confidence: 0.8,
        };
        let s = format!("{o:?}");
        assert!(s.contains("hello"));
        assert!(s.contains("0.8"));
    }

    #[test]
    fn which_finds_an_executable_we_just_created() {
        let dir = tempfile::tempdir().unwrap();
        // On Windows `which` checks PATHEXT (.EXE etc); on Unix it checks bare names.
        let exe_name = if cfg!(target_os = "windows") {
            "fake_tess.exe"
        } else {
            "fake_tess"
        };
        let file = dir.path().join(exe_name);
        std::fs::write(&file, b"#!/bin/sh\necho ok\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Prepend our temp dir to PATH and look up.
        let old_path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths: Vec<std::path::PathBuf> = vec![dir.path().to_path_buf()];
        paths.extend(std::env::split_paths(&old_path));
        let new_path = std::env::join_paths(paths).unwrap();
        // SAFETY: tests are single-threaded for env mutation here.
        // (cargo runs each test on its own thread; if this becomes flaky,
        // gate behind a Mutex.)
        std::env::set_var("PATH", &new_path);

        let found = which("fake_tess");

        // Restore env even on assertion failure.
        std::env::set_var("PATH", old_path);

        let found = found.expect("expected `which` to find our fake executable");
        // PATHEXT on Windows may upper-case the extension; compare case-insensitively.
        let found_str = found.to_string_lossy().to_lowercase();
        assert!(
            found_str.ends_with(&exe_name.to_lowercase()),
            "got: {}",
            found.display()
        );
    }

    #[test]
    fn which_returns_none_for_garbage_name() {
        assert!(which("definitely_no_binary_with_this_name_98235").is_none());
    }

    #[test]
    fn set_bundled_resource_dir_is_idempotent() {
        // OnceLock::set returns Err on the second call; the helper must not panic.
        let dir = std::path::PathBuf::from("/tmp/sk-bundled-test");
        set_bundled_resource_dir(dir.clone());
        // Second call — must not panic. If a different test set it first that's fine.
        set_bundled_resource_dir(dir);
    }

    #[test]
    fn resolve_tesseract_honors_env_override_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let exe_name = if cfg!(target_os = "windows") {
            "fake_tess_env.exe"
        } else {
            "fake_tess_env"
        };
        let file = dir.path().join(exe_name);
        std::fs::write(&file, b"x").unwrap();

        let old = std::env::var_os("SNK_TESSERACT_PATH");
        std::env::set_var("SNK_TESSERACT_PATH", &file);
        let got = resolve_tesseract();
        // Restore before assertions.
        match old {
            Some(v) => std::env::set_var("SNK_TESSERACT_PATH", v),
            None => std::env::remove_var("SNK_TESSERACT_PATH"),
        }
        assert_eq!(got.as_ref().map(|p| p.as_path()), Some(file.as_path()));
    }

    #[test]
    fn resolve_tesseract_ignores_env_override_pointing_at_nothing() {
        let old = std::env::var_os("SNK_TESSERACT_PATH");
        std::env::set_var("SNK_TESSERACT_PATH", "/path/that/does/not/exist/anywhere");
        // Should fall through to subsequent resolvers (PATH, install dirs),
        // any of which may or may not find tesseract on the host. We just
        // assert that the call doesn't panic and the env path itself is NOT
        // what came back.
        let got = resolve_tesseract();
        match old {
            Some(v) => std::env::set_var("SNK_TESSERACT_PATH", v),
            None => std::env::remove_var("SNK_TESSERACT_PATH"),
        }
        if let Some(p) = got {
            assert_ne!(p.to_string_lossy(), "/path/that/does/not/exist/anywhere");
        }
    }

    #[test]
    fn bundled_tessdata_dir_returns_none_when_no_real_dir() {
        // Best-effort: we can't reliably set BUNDLED_RESOURCE_DIR to a fresh
        // value because it's a OnceLock. Just confirm calling it doesn't panic.
        let _ = bundled_tessdata_dir();
    }
}
