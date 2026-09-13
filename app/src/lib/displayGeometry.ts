import { LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize } from '@tauri-apps/api/dpi';
import type { ScreenPreview } from '@snk/capture';
import type { CaretPosition } from '@snk/clipboard';

interface MonitorGeometry {
  position: { x: number; y: number };
  size: { width: number; height: number };
  scaleFactor: number;
}

export function overlayGeometry(frame: ScreenPreview['display']['frame']) {
  return frame.coordinateSpace === 'logical'
    ? {
        position: new LogicalPosition(frame.x, frame.y),
        size: new LogicalSize(frame.width, frame.height),
      }
    : {
        position: new PhysicalPosition(frame.x, frame.y),
        size: new PhysicalSize(frame.width, frame.height),
      };
}

export function clipboardPosition(anchor: CaretPosition, monitors: MonitorGeometry[]) {
  const logical = anchor.coordinateSpace === 'logical';
  const Position = logical ? LogicalPosition : PhysicalPosition;
  const frames = monitors
    .filter((m) => Number.isFinite(m.scaleFactor) && m.scaleFactor > 0)
    .map((m) => {
      // Tao's macOS monitor frames are scaled by each monitor's backing factor.
      // CGEvent anchors already use global logical points and must stay unscaled.
      const divisor = logical ? m.scaleFactor : 1;
      return {
        x: m.position.x / divisor,
        y: m.position.y / divisor,
        width: m.size.width / divisor,
        height: m.size.height / divisor,
        scale: logical ? 1 : m.scaleFactor,
      };
    });
  const monitor =
    frames.find(
      (m) =>
        anchor.x >= m.x && anchor.x < m.x + m.width && anchor.y >= m.y && anchor.y < m.y + m.height,
    ) ?? frames[0];
  if (!monitor) return new Position(0, 0);
  const { x: left, y: top, width, height, scale } = monitor;
  const pad = 8 * scale;
  const popupW = 380 * scale;
  const popupH = 480 * scale;
  // Preserve the existing dock/taskbar reservation until a native work-area
  // contract exists. Small displays keep the filter at their visible top edge.
  const bottom = top + height - 50 * scale;
  const desiredY = anchor.y + pad + popupH > bottom ? anchor.y - popupH - pad : anchor.y + pad;
  const x = Math.max(left + pad, Math.min(anchor.x, left + width - popupW - pad));
  const y = Math.max(top + pad, Math.min(desiredY, bottom - popupH));
  return new Position(x, y);
}
