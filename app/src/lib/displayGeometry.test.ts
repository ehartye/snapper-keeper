import { describe, expect, it } from 'vitest';
import { LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize } from '@tauri-apps/api/dpi';
import { clipboardPosition, overlayGeometry } from './displayGeometry';

const monitor = (x: number, y: number, width: number, height: number, scaleFactor: number) => ({
  position: { x, y },
  size: { width, height },
  scaleFactor,
});

describe('native display geometry', () => {
  it('retains logical origins across 2x to 1x and 1x to 2x moves', () => {
    for (const frame of [
      { coordinateSpace: 'logical' as const, x: -1440, y: 100, width: 1440, height: 900 },
      { coordinateSpace: 'logical' as const, x: 1728, y: 0, width: 1920, height: 1080 },
      { coordinateSpace: 'logical' as const, x: 0, y: 0, width: 1728, height: 1117 },
    ]) {
      const geometry = overlayGeometry(frame);
      expect(geometry.position).toBeInstanceOf(LogicalPosition);
      expect(geometry.size).toBeInstanceOf(LogicalSize);
      expect(geometry.position).toEqual(new LogicalPosition(frame.x, frame.y));
      expect(geometry.size).toEqual(new LogicalSize(frame.width, frame.height));
    }
  });

  it('retains Windows physical origins and sizes', () => {
    const geometry = overlayGeometry({
      coordinateSpace: 'physical',
      x: 2560,
      y: -100,
      width: 3840,
      height: 2160,
    });
    expect(geometry.position).toBeInstanceOf(PhysicalPosition);
    expect(geometry.size).toBeInstanceOf(PhysicalSize);
    expect(geometry.position).toEqual(new PhysicalPosition(2560, -100));
  });
});

describe('clipboard placement units', () => {
  it('does not halve a logical Mac cursor on Retina', () => {
    const position = clipboardPosition({ x: 1000, y: 100, coordinateSpace: 'logical' }, [
      monitor(0, 0, 3456, 2234, 2),
    ]);
    expect(position).toBeInstanceOf(LogicalPosition);
    expect(position).toEqual(new LogicalPosition(1000, 108));
  });

  it('selects a 1x external screen at logical 1728 independently of primary scale', () => {
    for (const scale of [1, 2]) {
      const position = clipboardPosition({ x: 1800, y: 100, coordinateSpace: 'logical' }, [
        monitor(0, 0, 1728 * scale, 1117 * scale, scale),
        monitor(1728, 0, 1920, 1080, 1),
      ]);
      expect(position).toEqual(new LogicalPosition(1800, 108));
    }
  });

  it('normalizes negative Retina monitor origins by their own scale', () => {
    const position = clipboardPosition({ x: -1400, y: 120, coordinateSpace: 'logical' }, [
      monitor(0, 0, 1920, 1080, 1),
      monitor(-2880, 200, 2880, 1800, 2),
    ]);
    expect(position).toEqual(new LogicalPosition(-1400, 128));
  });

  it('scales Windows popup dimensions and padding exactly once on the destination', () => {
    const position = clipboardPosition({ x: 2000, y: 100, coordinateSpace: 'physical' }, [
      monitor(0, 0, 1920, 1080, 1),
      monitor(1920, 0, 1920, 1080, 1.5),
    ]);
    expect(position).toBeInstanceOf(PhysicalPosition);
    expect(position).toEqual(new PhysicalPosition(2000, 112));
    expect(
      clipboardPosition({ x: 3800, y: 1000, coordinateSpace: 'physical' }, [
        monitor(1920, 0, 1920, 1080, 1.5),
      ]),
    ).toEqual(new PhysicalPosition(3258, 268));
  });

  it('keeps the filter visible on undersized displays and handles no monitor', () => {
    expect(
      clipboardPosition({ x: 100, y: 100, coordinateSpace: 'logical' }, [
        monitor(0, 0, 200, 200, 1),
      ]),
    ).toEqual(new LogicalPosition(8, 8));
    expect(clipboardPosition({ x: 100, y: 100, coordinateSpace: 'physical' }, [])).toEqual(
      new PhysicalPosition(0, 0),
    );
  });
});
