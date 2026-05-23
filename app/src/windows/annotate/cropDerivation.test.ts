import { describe, it, expect } from 'vitest';

import type { AnnotationShape } from '@snk/annotate';

import { shiftShapesForCrop } from './cropDerivation';

const baseStroke = { color: '#000', width: 2, opacity: 1 };

describe('shiftShapesForCrop', () => {
  it('translates box-positioned shapes by the negative crop offset', () => {
    const rect: AnnotationShape = {
      id: 'r-1',
      tool: 'rectangle',
      x: 150,
      y: 200,
      width: 30,
      height: 40,
      stroke: baseStroke,
    };
    const out = shiftShapesForCrop([rect], { x: 100, y: 50 });
    expect(out).toHaveLength(1);
    expect(out[0]!.x).toBe(50);
    expect(out[0]!.y).toBe(150);
    expect(out[0]!.width).toBe(30);
    expect(out[0]!.height).toBe(40);
  });

  it('translates point-positioned shapes pairwise', () => {
    const arrow: AnnotationShape = {
      id: 'a-1',
      tool: 'arrow',
      points: [120, 80, 200, 90, 250, 100],
      stroke: baseStroke,
    };
    const out = shiftShapesForCrop([arrow], { x: 100, y: 50 });
    expect(out[0]!.points).toEqual([20, 30, 100, 40, 150, 50]);
  });

  it('preserves stroke and unrelated fields', () => {
    const text: AnnotationShape = {
      id: 't-1',
      tool: 'text',
      x: 110,
      y: 70,
      text: 'hello',
      stroke: { color: '#ff0000', width: 4, opacity: 0.8 },
    };
    const out = shiftShapesForCrop([text], { x: 100, y: 50 });
    expect(out[0]!.text).toBe('hello');
    expect(out[0]!.stroke).toEqual({ color: '#ff0000', width: 4, opacity: 0.8 });
    expect(out[0]!.id).toBe('t-1');
    expect(out[0]!.tool).toBe('text');
  });

  it('returns a new array without mutating the input', () => {
    const rect: AnnotationShape = {
      id: 'r-2',
      tool: 'rectangle',
      x: 100,
      y: 100,
      stroke: baseStroke,
    };
    const input = [rect];
    const out = shiftShapesForCrop(input, { x: 10, y: 20 });
    expect(out).not.toBe(input);
    expect(out[0]).not.toBe(rect);
    expect(rect.x).toBe(100); // input untouched
    expect(rect.y).toBe(100);
  });
});
