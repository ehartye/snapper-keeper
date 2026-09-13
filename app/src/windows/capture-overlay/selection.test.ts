import { describe, expect, it } from 'vitest';
import { selectionPixels } from './selection';

describe('rendered preview selection', () => {
  it.each([1, 1.5, 2])('maps actual PNG ratio %s without devicePixelRatio', (ratio) => {
    expect(
      selectionPixels(
        { startX: 20, startY: 30, endX: 60, endY: 70 },
        { left: 10, top: 20, width: 200, height: 100 },
        200 * ratio,
        100 * ratio,
      ),
    ).toEqual({ x: 10 * ratio, y: 10 * ratio, w: 40 * ratio, h: 40 * ratio });
  });

  it('rounds endpoints outward for fractional reverse drags', () => {
    expect(
      selectionPixels(
        { startX: 60.2, startY: 40.1, endX: 20.4, endY: 20.2 },
        { left: 10, top: 10, width: 200, height: 100 },
        300,
        150,
      ),
    ).toEqual({ x: 15, y: 15, w: 61, h: 31 });
  });

  it('clips selections to the full rendered image', () => {
    expect(
      selectionPixels(
        { startX: -100, startY: -200, endX: 1000, endY: 2000 },
        { left: 10, top: 10, width: 200, height: 100 },
        300,
        150,
      ),
    ).toEqual({ x: 0, y: 0, w: 300, h: 150 });
  });

  it('rejects zero dimensions, nonfinite input, and wholly outside drags', () => {
    const bounds = { left: 10, top: 10, width: 200, height: 100 };
    const selection = { startX: 20, startY: 20, endX: 40, endY: 40 };
    expect(selectionPixels(selection, { ...bounds, width: 0 }, 300, 150)).toBeNull();
    expect(selectionPixels(selection, bounds, 0, 150)).toBeNull();
    expect(
      selectionPixels({ ...selection, startX: 20.2, endX: 20.2 }, bounds, 300, 150),
    ).toBeNull();
    expect(selectionPixels({ ...selection, startX: NaN }, bounds, 300, 150)).toBeNull();
    expect(selectionPixels(selection, { ...bounds, top: Infinity }, 300, 150)).toBeNull();
    expect(
      selectionPixels({ startX: 500, startY: 500, endX: 600, endY: 600 }, bounds, 300, 150),
    ).toBeNull();
  });
});
