use crate::{display::Display, foreground::ForegroundInfo, grab::GrabResult, CaptureError, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScreenPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub token: String,
    pub display: Display,
}

struct Snapshot {
    token: String,
    pixels: GrabResult,
    foreground: Option<ForegroundInfo>,
}

/// A single bounded snapshot. Access is owned by CaptureWorker's cancellation-safe
/// gate, including file replacement, crop, persistence and event delivery.
#[derive(Default)]
pub struct PreviewSession {
    snapshot: Option<Snapshot>,
}

impl PreviewSession {
    pub fn replace(
        &mut self,
        token: String,
        pixels: GrabResult,
        foreground: Option<ForegroundInfo>,
    ) {
        self.snapshot = Some(Snapshot {
            token,
            pixels,
            foreground,
        });
    }

    pub fn crop(
        &mut self,
        token: &str,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> Result<(GrabResult, Option<ForegroundInfo>)> {
        let snapshot = self
            .snapshot
            .as_ref()
            .filter(|s| s.token == token)
            .ok_or(CaptureError::StalePreview)?;
        let image = image::load_from_memory(&snapshot.pixels.png_bytes)?.to_rgba8();
        let (x, y, w, h) = crate::grab::clamp_region(image.width(), image.height(), x, y, w, h)
            .ok_or_else(|| CaptureError::Os {
                message: "region has zero area".into(),
            })?;
        let cropped = image::imageops::crop_imm(&image, x, y, w, h).to_image();
        let png_bytes = crate::grab::encode_rgba_to_png(cropped.as_raw(), w, h)?;
        let snapshot = self.snapshot.take().expect("validated snapshot");
        Ok((
            GrabResult {
                png_bytes,
                width: w,
                height: h,
                monitor_name: snapshot.pixels.monitor_name,
            },
            snapshot.foreground,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_wire_keeps_native_geometry_separate_from_png_pixels() {
        let preview = ScreenPreview {
            path: "captures/.preview.png".into(),
            token: "A".into(),
            width: 2880,
            height: 1800,
            display: Display {
                id: 77,
                frame: crate::display::DisplayFrame {
                    coordinate_space: "logical",
                    x: -1440,
                    y: 100,
                    width: 1440,
                    height: 900,
                },
            },
        };
        let wire = serde_json::to_value(preview).unwrap();
        assert_eq!(wire["width"], 2880);
        assert_eq!(wire["display"]["id"], 77);
        assert_eq!(wire["display"]["frame"]["coordinateSpace"], "logical");
        assert_eq!(wire["display"]["frame"]["x"], -1440);
        assert_eq!(wire["display"]["frame"]["width"], 1440);
        assert_eq!(
            serde_json::to_value(CaptureError::StalePreview).unwrap()["kind"],
            "stale-preview"
        );
        assert_eq!(
            serde_json::to_value(CaptureError::MonitorNotFound { id: 42 }).unwrap(),
            serde_json::json!({ "kind": "monitor-not-found", "data": { "id": 42 } })
        );
    }

    fn pixels() -> GrabResult {
        GrabResult {
            png_bytes: crate::grab::encode_rgba_to_png(&[255, 0, 0, 255, 0, 255, 0, 255], 2, 1)
                .unwrap(),
            width: 2,
            height: 1,
            monitor_name: "unplugged display".into(),
        }
    }

    #[test]
    fn crops_owned_snapshot_without_a_monitor_and_consumes_token() {
        let mut session = PreviewSession::default();
        let mut source = pixels();
        session.replace(
            "A".into(),
            source.clone(),
            Some(ForegroundInfo {
                app_name: "Original".into(),
                window_title: "Before overlay".into(),
            }),
        );
        source.png_bytes =
            crate::grab::encode_rgba_to_png(&[0, 0, 255, 255, 0, 0, 255, 255], 2, 1).unwrap();
        assert_ne!(source.png_bytes, pixels().png_bytes);
        let (crop, fg) = session.crop("A", 1, 0, 1, 1).unwrap();
        assert_eq!(
            image::load_from_memory(&crop.png_bytes)
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)
                .0,
            [0, 255, 0, 255]
        );
        assert_eq!(crop.monitor_name, "unplugged display");
        assert_eq!(fg.unwrap().app_name, "Original");
        assert!(matches!(
            session.crop("A", 0, 0, 1, 1),
            Err(CaptureError::StalePreview)
        ));
    }

    #[test]
    fn newer_preview_supersedes_old_without_consuming_current_on_stale_request() {
        let mut session = PreviewSession::default();
        session.replace("A".into(), pixels(), None);
        session.replace("B".into(), pixels(), None);
        assert!(matches!(
            session.crop("A", 0, 0, 1, 1),
            Err(CaptureError::StalePreview)
        ));
        assert!(session.crop("B", 0, 0, 1, 1).is_ok());
    }
}
