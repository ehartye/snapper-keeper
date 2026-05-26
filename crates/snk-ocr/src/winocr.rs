#![cfg(target_os = "windows")]

use std::path::Path;

use tracing::debug;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::{FileAccessMode, StorageFile};
use windows::core::HSTRING;

use crate::OcrError;
use crate::backend::{BBox, OcrBackend, OcrResult, OcrWord};

pub struct WinOcrBackend {
    engine: OcrEngine,
}

impl WinOcrBackend {
    pub fn new() -> Result<Self, OcrError> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().or_else(|_| {
            let en =
                Language::CreateLanguage(&HSTRING::from("en-US")).map_err(|e| {
                    OcrError::NoRecognizerLanguage {
                        detail: format!("CreateLanguage(en-US): {e}"),
                    }
                })?;
            OcrEngine::TryCreateFromLanguage(&en).map_err(|e| OcrError::NoRecognizerLanguage {
                detail: format!("TryCreateFromLanguage(en-US): {e}"),
            })
        })?;
        Ok(Self { engine })
    }
}

impl OcrBackend for WinOcrBackend {
    fn name(&self) -> &'static str {
        "Windows.Media.Ocr"
    }

    fn engine_version(&self) -> String {
        let build = win_build_number().unwrap_or_else(|| "unknown".to_string());
        format!("Windows.Media.Ocr (10.0.{build})")
    }

    fn recognize(&self, image_path: &Path) -> Result<OcrResult, OcrError> {
        let abs = image_path.canonicalize().map_err(|e| OcrError::ImageLoad {
            path: image_path.display().to_string(),
            detail: e.to_string(),
        })?;
        // Plan amendment (Spike A finding approved 2026-05-26): Windows
        // canonicalize() returns extended-length namespace paths prefixed with
        // \\?\, which StorageFile::GetFileFromPathAsync rejects with HRESULT
        // 0x800700A1. Strip the prefix before constructing the HSTRING.
        let abs_str = abs.to_string_lossy();
        let stripped = abs_str.strip_prefix(r"\\?\").unwrap_or(&abs_str);
        let path = HSTRING::from(stripped);

        // Plan amendment (Spike A finding approved 2026-05-26): windows-rs 0.62
        // removed `IAsyncOperation::get()`. Use the inherent `.join()` method
        // on each of the four async types — no `windows-future` dep, no trait import.
        let file = StorageFile::GetFileFromPathAsync(&path)
            .map_err(|e| OcrError::ImageLoad {
                path: image_path.display().to_string(),
                detail: format!("GetFileFromPathAsync: {e}"),
            })?
            .join()
            .map_err(|e| OcrError::ImageLoad {
                path: image_path.display().to_string(),
                detail: format!("await GetFileFromPathAsync: {e}"),
            })?;

        let stream = file
            .OpenAsync(FileAccessMode::Read)
            .and_then(|op| op.join())
            .map_err(|e| OcrError::ImageLoad {
                path: image_path.display().to_string(),
                detail: format!("OpenAsync: {e}"),
            })?;

        let decoder = BitmapDecoder::CreateAsync(&stream)
            .and_then(|op| op.join())
            .map_err(|e| OcrError::ImageLoad {
                path: image_path.display().to_string(),
                detail: format!("BitmapDecoder: {e}"),
            })?;

        let pixel_width = decoder.PixelWidth().unwrap_or(1) as f32;
        let pixel_height = decoder.PixelHeight().unwrap_or(1) as f32;

        let bitmap = decoder
            .GetSoftwareBitmapAsync()
            .and_then(|op| op.join())
            .map_err(|e| OcrError::ImageLoad {
                path: image_path.display().to_string(),
                detail: format!("GetSoftwareBitmap: {e}"),
            })?;

        let result = self
            .engine
            .RecognizeAsync(&bitmap)
            .and_then(|op| op.join())
            .map_err(|e| OcrError::Recognize {
                detail: format!("RecognizeAsync: {e}"),
            })?;

        let lines = result.Lines().map_err(|e| OcrError::Recognize {
            detail: format!("Lines: {e}"),
        })?;
        let line_count = lines.Size().unwrap_or(0);

        let mut text_lines: Vec<String> = Vec::new();
        let mut words: Vec<OcrWord> = Vec::new();

        // Plan amendment (Spike A finding approved 2026-05-26): Windows.Media.Ocr
        // does NOT expose any Confidence accessor on OcrWord OR OcrLine (verified
        // by reading windows-0.62.2 source). Hard-code line-broadcast confidence
        // at 0.85 — this is the documented asymmetry vs Vision.
        const WIN_OCR_HEURISTIC_CONF: f64 = 0.85;

        for li in 0..line_count {
            let line = match lines.GetAt(li) {
                Ok(l) => l,
                Err(e) => {
                    debug!("line {li} GetAt err: {e}");
                    continue;
                }
            };
            let line_text = line.Text().map(|h| h.to_string_lossy()).unwrap_or_default();
            text_lines.push(line_text.clone());

            let line_words = line.Words().map_err(|e| OcrError::Recognize {
                detail: format!("Words: {e}"),
            })?;
            for wi in 0..line_words.Size().unwrap_or(0) {
                let w = match line_words.GetAt(wi) {
                    Ok(w) => w,
                    Err(_) => continue,
                };
                let wt = w.Text().map(|h| h.to_string_lossy()).unwrap_or_default();
                let r = match w.BoundingRect() {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let bbox = BBox {
                    x: r.X / pixel_width,
                    y: r.Y / pixel_height,
                    w: r.Width / pixel_width,
                    h: r.Height / pixel_height,
                };
                words.push(OcrWord {
                    text: wt,
                    bbox,
                    confidence: WIN_OCR_HEURISTIC_CONF,
                    line: li,
                });
            }
        }

        let text = text_lines.join("\n");
        let language = self
            .engine
            .RecognizerLanguage()
            .ok()
            .and_then(|l| l.LanguageTag().ok().map(|h| h.to_string_lossy()))
            .unwrap_or_else(|| "auto".to_string());

        Ok(OcrResult {
            text,
            words,
            language,
            confidence: WIN_OCR_HEURISTIC_CONF,
        })
    }
}

fn win_build_number() -> Option<String> {
    // Followup to plan §T8: the plan-literal `std::env::var("OS_BUILD")` path was
    // wrong — Windows doesn't set `OS_BUILD`, so engine_version() always read
    // "unknown". RtlGetVersion is the authoritative source (ntdll, not subject
    // to the GetVersionEx app-compat lying that Microsoft bolted on in 8.1+).
    // Returns NTSTATUS == 0 (STATUS_SUCCESS) on success.
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    let status = unsafe { RtlGetVersion(&mut info) };
    if status.0 == 0 {
        Some(info.dwBuildNumber.to_string())
    } else {
        None
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn winocr_backend_constructs_and_reports_version() {
        match WinOcrBackend::new() {
            Ok(b) => {
                assert_eq!(b.name(), "Windows.Media.Ocr");
                let v = b.engine_version();
                assert!(v.starts_with("Windows.Media.Ocr ("));
                // Confirm RtlGetVersion succeeded — `unknown` is the documented
                // fall-back when the syscall returns non-zero NTSTATUS, which
                // shouldn't happen on a normal Windows host.
                assert!(
                    !v.contains("unknown"),
                    "expected real build number from RtlGetVersion, got: {v}"
                );
            }
            Err(OcrError::NoRecognizerLanguage { detail }) => {
                eprintln!("test machine has no recognizer language available: {detail}");
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }
}
