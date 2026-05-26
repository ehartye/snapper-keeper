//! Input validation for IPC commands. Cheap guards against malformed
//! or oversized payloads landing in the library before any disk-write.

use serde::Deserialize;

use crate::AnnotateError;

/// Maximum accepted size for a PNG payload (16 MiB). Generous enough
/// for full-screen 8K screenshots at 32bpp (~33 MiB uncompressed, but
/// PNG-compressed in real-world content usually 2-8 MiB) without
/// allowing arbitrary multi-hundred-MB attacker-controlled allocation.
pub const MAX_PNG_BYTES: usize = 16 * 1024 * 1024;

/// Maximum accepted size for a serialized `AnnotationState` JSON
/// payload (1 MiB). Annotation state holds a small array of shape
/// descriptors + crop coords; legitimate payloads are <10 KB. The
/// 1 MiB ceiling tolerates fragile editors that splatter heavy
/// metadata while still rejecting obvious abuse.
pub const MAX_STATE_JSON_BYTES: usize = 1024 * 1024;

/// PNG file signature (first 8 bytes of every well-formed PNG).
/// See RFC 2083 §3.1.
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Validate that `bytes` start with the PNG file signature and don't
/// exceed `MAX_PNG_BYTES`. Returns the byte count for the caller to
/// log if useful.
pub fn validate_png_payload(bytes: &[u8]) -> Result<usize, AnnotateError> {
    if bytes.len() > MAX_PNG_BYTES {
        return Err(AnnotateError::InvalidInput {
            reason: format!(
                "PNG payload too large: {} bytes (max {} bytes / 16 MiB)",
                bytes.len(),
                MAX_PNG_BYTES
            ),
        });
    }
    if bytes.len() < PNG_SIGNATURE.len() {
        return Err(AnnotateError::InvalidInput {
            reason: format!(
                "payload too short to be a PNG: {} bytes (need >= 8 for signature)",
                bytes.len()
            ),
        });
    }
    if bytes[..PNG_SIGNATURE.len()] != PNG_SIGNATURE {
        return Err(AnnotateError::InvalidInput {
            reason: "payload does not start with the PNG signature".into(),
        });
    }
    Ok(bytes.len())
}

/// Schema for the `state_json` field accepted by annotate commands.
/// Mirrors the TypeScript `AnnotationState` in
/// `packages/snk-annotate/src/index.ts`. Inner `shapes` array stays
/// opaque (`serde_json::Value`) because shape descriptors are
/// frontend-managed; we validate that the array exists and the wrapper
/// shape is correct, not the shape geometry itself.
#[derive(Debug, Deserialize)]
pub struct AnnotationState {
    pub version: u32,
    pub shapes: Vec<serde_json::Value>,
    pub crop_region: Option<CropRegion>,
    pub crop_confirmed: bool,
}

#[derive(Debug, Deserialize)]
pub struct CropRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Validate `state_json` parses as an `AnnotationState` with a known
/// `version`. Returns the parsed struct so the caller can re-use the
/// already-deserialized form if desired (currently the commands write
/// the original string verbatim, but future code paths may want the
/// typed value).
pub fn validate_annotation_state(state_json: &str) -> Result<AnnotationState, AnnotateError> {
    if state_json.len() > MAX_STATE_JSON_BYTES {
        return Err(AnnotateError::InvalidInput {
            reason: format!(
                "state_json too large: {} bytes (max {} bytes / 1 MiB)",
                state_json.len(),
                MAX_STATE_JSON_BYTES
            ),
        });
    }
    let parsed: AnnotationState =
        serde_json::from_str(state_json).map_err(|e| AnnotateError::InvalidInput {
            reason: format!("state_json failed to parse: {e}"),
        })?;
    // Currently only version 1 is accepted. When the schema evolves,
    // either accept multiple versions here (with backward-compat) or
    // bump the constant and update the TS binding's literal type.
    if parsed.version != 1 {
        return Err(AnnotateError::InvalidInput {
            reason: format!(
                "state_json version {} not supported (expected 1)",
                parsed.version
            ),
        });
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_png() -> Vec<u8> {
        // 8-byte signature + 4-byte IHDR length (0) + IHDR type — minimal
        // signature-bearing buffer. Real validation of PNG contents
        // happens at the image decoder; this validator is structural only.
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(&[0; 16]);
        bytes
    }

    #[test]
    fn png_signature_accepts_valid_signature() {
        let bytes = valid_png();
        assert_eq!(validate_png_payload(&bytes).unwrap(), bytes.len());
    }

    #[test]
    fn png_signature_rejects_wrong_magic() {
        let bytes = b"GIF89a; not a png".to_vec();
        let err = validate_png_payload(&bytes).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("PNG signature"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn png_signature_rejects_too_short_buffer() {
        let bytes = vec![0x89, 0x50, 0x4E]; // 3 bytes, can't even contain signature
        let err = validate_png_payload(&bytes).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("too short"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn png_payload_rejects_oversized() {
        let bytes = vec![0u8; MAX_PNG_BYTES + 1];
        let err = validate_png_payload(&bytes).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("too large"));
                assert!(reason.contains("16 MiB"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn png_payload_accepts_exact_max_size() {
        let mut bytes = vec![0u8; MAX_PNG_BYTES];
        bytes[..PNG_SIGNATURE.len()].copy_from_slice(&PNG_SIGNATURE);
        assert_eq!(validate_png_payload(&bytes).unwrap(), MAX_PNG_BYTES);
    }

    #[test]
    fn annotation_state_accepts_minimal_valid_state() {
        let json = r#"{"version": 1, "shapes": [], "crop_region": null, "crop_confirmed": false}"#;
        let s = validate_annotation_state(json).unwrap();
        assert_eq!(s.version, 1);
        assert!(s.shapes.is_empty());
        assert!(s.crop_region.is_none());
        assert!(!s.crop_confirmed);
    }

    #[test]
    fn annotation_state_accepts_state_with_crop_and_shapes() {
        let json = r#"{
            "version": 1,
            "shapes": [{"kind": "rect", "x": 10, "y": 20}, {"kind": "arrow"}],
            "crop_region": {"x": 0, "y": 0, "width": 100, "height": 200},
            "crop_confirmed": true
        }"#;
        let s = validate_annotation_state(json).unwrap();
        assert_eq!(s.shapes.len(), 2);
        let crop = s.crop_region.unwrap();
        assert_eq!(crop.width, 100.0);
        assert_eq!(crop.height, 200.0);
        assert!(s.crop_confirmed);
    }

    #[test]
    fn annotation_state_rejects_malformed_json() {
        let json = r#"{"version": 1, "shapes":"#; // truncated
        let err = validate_annotation_state(json).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("failed to parse"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn annotation_state_rejects_missing_required_field() {
        let json = r#"{"version": 1, "shapes": []}"#; // missing crop_region + crop_confirmed
        let err = validate_annotation_state(json).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("failed to parse"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn annotation_state_rejects_unknown_version() {
        let json = r#"{"version": 2, "shapes": [], "crop_region": null, "crop_confirmed": false}"#;
        let err = validate_annotation_state(json).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("version 2"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn annotation_state_rejects_oversized_json() {
        // Just over the limit. Content doesn't need to be valid JSON; the
        // size check fires first.
        let payload = "x".repeat(MAX_STATE_JSON_BYTES + 1);
        let err = validate_annotation_state(&payload).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("too large"));
                assert!(reason.contains("1 MiB"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn annotation_state_rejects_wrong_field_types() {
        // crop_confirmed is the wrong type
        let json = r#"{"version": 1, "shapes": [], "crop_region": null, "crop_confirmed": "yes"}"#;
        let err = validate_annotation_state(json).unwrap_err();
        match err {
            AnnotateError::InvalidInput { reason } => {
                assert!(reason.contains("failed to parse"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }
}
