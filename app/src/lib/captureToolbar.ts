import { PhysicalPosition } from '@tauri-apps/api/dpi';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { availableMonitors, cursorPosition } from '@tauri-apps/api/window';
import { captureCursorPosition } from '@snk/capture';

const TOOLBAR_LOGICAL_WIDTH = 420;
const TOOLBAR_LOGICAL_HEIGHT = 56;
const TOOLBAR_LOGICAL_PAD = 12;

type MonitorLike = {
  position: { x: number; y: number };
  size: { width: number; height: number };
  scaleFactor: number;
};

type Point = { x: number; y: number };

function isMacPlatform() {
  if (typeof navigator === 'undefined') return false;
  const uaDataPlatform = (
    navigator as Navigator & { userAgentData?: { platform?: string } }
  ).userAgentData?.platform;
  return (
    /mac/i.test(uaDataPlatform ?? '') ||
    /Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
    /\bMac OS\b/.test(navigator.userAgent)
  );
}

function monitorForPoint<T extends MonitorLike>(monitors: T[], point: Point): T | undefined {
  return (
    monitors.find(
      (monitor) =>
        point.x >= monitor.position.x &&
        point.x < monitor.position.x + monitor.size.width &&
        point.y >= monitor.position.y &&
        point.y < monitor.position.y + monitor.size.height,
    ) ?? monitors[0]
  );
}

export function toolbarPositionForCursor<T extends MonitorLike>(monitors: T[], cursor: Point) {
  const monitor = monitorForPoint(monitors, cursor);
  if (!monitor) {
    return { x: 0, y: 0 };
  }

  const scaleFactor = monitor.scaleFactor;
  const toolbarWidth = TOOLBAR_LOGICAL_WIDTH * scaleFactor;
  const toolbarHeight = TOOLBAR_LOGICAL_HEIGHT * scaleFactor;
  const toolbarPad = TOOLBAR_LOGICAL_PAD * scaleFactor;
  const monLeft = monitor.position.x;
  const monTop = monitor.position.y;
  const monRight = monLeft + monitor.size.width;
  const monBottom = monTop + monitor.size.height;

  let x = cursor.x - toolbarWidth / 2;
  let y = cursor.y + toolbarPad;

  if (y + toolbarHeight > monBottom) {
    y = cursor.y - toolbarHeight - toolbarPad;
  }
  if (x < monLeft + toolbarPad) {
    x = monLeft + toolbarPad;
  }
  if (x + toolbarWidth > monRight - toolbarPad) {
    x = monRight - toolbarWidth - toolbarPad;
  }
  if (y < monTop + toolbarPad) {
    y = monTop + toolbarPad;
  }
  if (y + toolbarHeight > monBottom - toolbarPad) {
    y = monBottom - toolbarHeight - toolbarPad;
  }

  return { x: Math.round(x), y: Math.round(y) };
}

export async function showCaptureToolbar(captureId: string) {
  const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
  if (!toolbar) {
    return;
  }

  const [cursor, monitors] = await Promise.all([
    isMacPlatform() ? captureCursorPosition() : cursorPosition(),
    availableMonitors(),
  ]);
  const { x, y } = toolbarPositionForCursor(monitors, cursor);
  await toolbar.setPosition(new PhysicalPosition(x, y));
  await toolbar.emit('toolbar:show', { captureId });
  await toolbar.show();
  await toolbar.setFocus();
}
