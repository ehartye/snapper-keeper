#![cfg(target_os = "macos")]

use std::path::Path;

use objc2::rc::Retained;
use objc2_foundation::{NSDictionary, NSRange, NSString, NSURL};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation,
    VNRequestTextRecognitionLevel,
};
use tracing::{debug, warn};

use crate::backend::{BBox, OcrBackend, OcrResult, OcrWord};
use crate::OcrError;

pub struct VisionBackend;

impl VisionBackend {
    pub fn new() -> Result<Self, OcrError> {
        Ok(Self)
    }
}

impl OcrBackend for VisionBackend {
    fn name(&self) -> &'static str { "Vision" }

    fn engine_version(&self) -> String {
        let v = unsafe {
            let pi = objc2_foundation::NSProcessInfo::processInfo();
            let os = pi.operatingSystemVersion();
            format!("{}.{}.{}", os.majorVersion, os.minorVersion, os.patchVersion)
        };
        format!("Vision (macOS {v})")
    }

    fn recognize(&self, image_path: &Path) -> Result<OcrResult, OcrError> {
        let abs = image_path.canonicalize().map_err(|e| OcrError::ImageLoad {
            path: image_path.display().to_string(),
            detail: e.to_string(),
        })?;

        unsafe {
            let path_str = NSString::from_str(&abs.to_string_lossy());
            let url = NSURL::fileURLWithPath(&path_str);

            let request = VNRecognizeTextRequest::new();
            request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
            request.setAutomaticallyDetectsLanguage(true);

            let handler = VNImageRequestHandler::initWithURL_options(
                VNImageRequestHandler::alloc(),
                &url,
                &NSDictionary::new(),
            );

            let perform_result = handler.performRequests_error(
                &objc2_foundation::NSArray::from_slice(&[
                    objc2::runtime::ProtocolObject::from_ref(&*request).cast()
                ])
            );
            perform_result.map_err(|e| OcrError::Recognize {
                detail: format!("performRequests: {e:?}"),
            })?;

            let observations = request.results().ok_or_else(|| OcrError::Recognize {
                detail: "results() returned nil".into(),
            })?;

            let mut text_lines: Vec<String> = Vec::new();
            let mut words: Vec<OcrWord> = Vec::new();
            let mut total_conf: f64 = 0.0;
            let mut conf_count: usize = 0;

            for line_idx in 0..observations.count() {
                let obs: Retained<VNRecognizedTextObservation> =
                    observations.objectAtIndex(line_idx).cast();
                let candidates = obs.topCandidates(1);
                if candidates.count() == 0 { continue; }
                let candidate = candidates.objectAtIndex(0);
                let line_text = candidate.string().to_string();
                let line_conf = candidate.confidence() as f64;

                total_conf += line_conf;
                conf_count += 1;
                text_lines.push(line_text.clone());

                let line_u32 = line_idx as u32;
                let candidate_str = candidate.string();
                let total_len = candidate_str.len();
                let mut byte_pos: usize = 0;
                for word in line_text.split_whitespace() {
                    let word_len = word.len();
                    let range = NSRange { location: byte_pos, length: word_len };
                    if byte_pos + word_len > total_len {
                        warn!("vision word range out of bounds; skipping");
                        break;
                    }
                    match candidate.boundingBoxForRange_error(range) {
                        Ok(rect_obs) => {
                            let r = rect_obs.boundingBox();
                            // Vision returns normalized 0..1 with origin BOTTOM-LEFT.
                            // Convert to TOP-LEFT for our schema convention.
                            let bbox = BBox {
                                x: r.origin.x as f32,
                                y: (1.0 - (r.origin.y + r.size.height)) as f32,
                                w: r.size.width as f32,
                                h: r.size.height as f32,
                            };
                            words.push(OcrWord {
                                text: word.to_string(),
                                bbox,
                                confidence: line_conf,
                                line: line_u32,
                            });
                        }
                        Err(e) => {
                            debug!("boundingBoxForRange failed for '{word}': {e:?}");
                        }
                    }
                    byte_pos += word_len + 1;
                }
            }

            let text = text_lines.join("\n");
            let avg_conf = if conf_count > 0 { total_conf / conf_count as f64 } else { 0.0 };

            Ok(OcrResult {
                text,
                words,
                language: "auto".to_string(),
                confidence: avg_conf,
            })
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn vision_backend_constructs_and_reports_version() {
        let b = VisionBackend::new().expect("construct");
        assert_eq!(b.name(), "Vision");
        let v = b.engine_version();
        assert!(v.starts_with("Vision (macOS "));
    }
}
