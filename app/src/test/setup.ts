// Vitest setup — runs before each test file.
//
// Mocks every @tauri-apps/api module the app uses so component tests can
// run in happy-dom without touching the real Tauri runtime. Individual
// tests override these mocks (vi.mocked(...).mockReturnValue(...)) when
// they care about a specific call.

import { vi, beforeEach } from 'vitest';
import '@testing-library/jest-dom/vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/window', () => {
  const fakeWindow = {
    label: 'library',
    show: vi.fn().mockResolvedValue(undefined),
    hide: vi.fn().mockResolvedValue(undefined),
    setFocus: vi.fn().mockResolvedValue(undefined),
    setPosition: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    emit: vi.fn().mockResolvedValue(undefined),
    onCloseRequested: vi.fn().mockResolvedValue(() => {}),
  };
  return {
    getCurrentWindow: () => fakeWindow,
    availableMonitors: vi.fn().mockResolvedValue([
      {
        name: 'Mock Monitor',
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        scaleFactor: 1,
      },
    ]),
  };
});

vi.mock('@tauri-apps/api/webviewWindow', () => {
  const fakeWebviewWindow = {
    label: 'mock',
    show: vi.fn().mockResolvedValue(undefined),
    hide: vi.fn().mockResolvedValue(undefined),
    setFocus: vi.fn().mockResolvedValue(undefined),
    setPosition: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    emit: vi.fn().mockResolvedValue(undefined),
  };
  return {
    WebviewWindow: {
      getByLabel: vi.fn().mockResolvedValue(fakeWebviewWindow),
    },
  };
});

vi.mock('@tauri-apps/api/dpi', () => ({
  LogicalPosition: class {
    constructor(public x: number, public y: number) {}
  },
  LogicalSize: class {
    constructor(public width: number, public height: number) {}
  },
}));

vi.mock('@tauri-apps/api', () => ({
  path: {
    appDataDir: vi.fn().mockResolvedValue('/mock/app/data'),
  },
}));

vi.mock('@tauri-apps/plugin-autostart', () => ({
  isEnabled: vi.fn().mockResolvedValue(false),
  enable: vi.fn().mockResolvedValue(undefined),
  disable: vi.fn().mockResolvedValue(undefined),
}));

// IntersectionObserver isn't implemented in happy-dom; provide a no-op
// stub so components that use it (CaptureGrid sentinel) don't crash.
class IntersectionObserverStub {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  takeRecords = vi.fn(() => []);
  root = null;
  rootMargin = '';
  thresholds = [];
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
(globalThis as any).IntersectionObserver = IntersectionObserverStub;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
(window as any).IntersectionObserver = IntersectionObserverStub;

// ResizeObserver same situation.
class ResizeObserverStub {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
(globalThis as any).ResizeObserver = ResizeObserverStub;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
(window as any).ResizeObserver = ResizeObserverStub;

// Reset all mocks between tests so call counts don't leak.
beforeEach(() => {
  vi.clearAllMocks();
});
