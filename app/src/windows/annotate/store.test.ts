import { describe, it, expect, beforeEach } from 'vitest';

import { useAnnotateStore, COLORS, STROKE_WIDTHS } from './store';
import type { AnnotationShape } from '@snk/annotate';

const stroke = { color: '#000', width: 2, opacity: 1 };

const shapeA: AnnotationShape = {
  id: 'sh-a',
  tool: 'rectangle',
  x: 0,
  y: 0,
  width: 10,
  height: 10,
  stroke,
};

const shapeB: AnnotationShape = {
  id: 'sh-b',
  tool: 'arrow',
  points: [0, 0, 10, 10],
  stroke,
};

describe('useAnnotateStore', () => {
  beforeEach(() => {
    useAnnotateStore.getState().reset();
  });

  it('exposes the COLORS and STROKE_WIDTHS constants', () => {
    expect(COLORS.length).toBeGreaterThanOrEqual(4);
    expect(STROKE_WIDTHS.thin).toBeLessThan(STROKE_WIDTHS.medium);
    expect(STROKE_WIDTHS.medium).toBeLessThan(STROKE_WIDTHS.thick);
  });

  it('defaults to the select tool', () => {
    expect(useAnnotateStore.getState().tool).toBe('select');
  });

  it('setTool clears the selection when switching off select', () => {
    const s = useAnnotateStore.getState();
    s.setSelectedId('sh-1');
    s.setTool('rectangle');
    expect(useAnnotateStore.getState().selectedId).toBeNull();
  });

  it('setTool keeps the selection when re-selecting the select tool', () => {
    const s = useAnnotateStore.getState();
    s.setSelectedId('sh-1');
    s.setTool('select');
    expect(useAnnotateStore.getState().selectedId).toBe('sh-1');
  });

  it('addShape appends and pushes the previous shapes onto the undo stack', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    expect(useAnnotateStore.getState().shapes).toHaveLength(1);
    s.addShape(shapeB);
    const after = useAnnotateStore.getState();
    expect(after.shapes).toHaveLength(2);
    expect(after.undoStack).toHaveLength(2);
    expect(after.redoStack).toHaveLength(0);
  });

  it('addShape advances nextStepNumber only for step-marker shapes', () => {
    const s = useAnnotateStore.getState();
    const before = useAnnotateStore.getState().nextStepNumber;
    s.addShape(shapeA);
    expect(useAnnotateStore.getState().nextStepNumber).toBe(before);
    s.addShape({
      id: 'sh-step',
      tool: 'step-marker',
      x: 0,
      y: 0,
      stepNumber: before,
      stroke,
    });
    expect(useAnnotateStore.getState().nextStepNumber).toBe(before + 1);
  });

  it('updateShape patches the matching shape', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.updateShape('sh-a', { x: 100, y: 100 });
    const updated = useAnnotateStore.getState().shapes[0]!;
    expect(updated.x).toBe(100);
    expect(updated.y).toBe(100);
    // and pushes undo
    expect(useAnnotateStore.getState().undoStack.length).toBeGreaterThanOrEqual(1);
  });

  it('deleteShape removes the shape and clears selectedId when it matches', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.addShape(shapeB);
    s.setSelectedId('sh-b');
    s.deleteShape('sh-b');
    const after = useAnnotateStore.getState();
    expect(after.shapes).toHaveLength(1);
    expect(after.shapes[0]!.id).toBe('sh-a');
    expect(after.selectedId).toBeNull();
  });

  it('deleteShape preserves selectedId when removing a different shape', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.addShape(shapeB);
    s.setSelectedId('sh-a');
    s.deleteShape('sh-b');
    expect(useAnnotateStore.getState().selectedId).toBe('sh-a');
  });

  it('undo/redo round-trip add operations', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.addShape(shapeB);
    s.undo();
    expect(useAnnotateStore.getState().shapes).toHaveLength(1);
    s.undo();
    expect(useAnnotateStore.getState().shapes).toHaveLength(0);
    s.redo();
    expect(useAnnotateStore.getState().shapes).toHaveLength(1);
    s.redo();
    expect(useAnnotateStore.getState().shapes).toHaveLength(2);
  });

  it('undo on an empty stack is a no-op', () => {
    const before = useAnnotateStore.getState().shapes;
    useAnnotateStore.getState().undo();
    expect(useAnnotateStore.getState().shapes).toBe(before);
  });

  it('redo on an empty stack is a no-op', () => {
    const before = useAnnotateStore.getState().shapes;
    useAnnotateStore.getState().redo();
    expect(useAnnotateStore.getState().shapes).toBe(before);
  });

  it('setCropRegion resets cropConfirmed; confirmCrop sets it and switches tool', () => {
    const s = useAnnotateStore.getState();
    s.setCropRegion({ x: 0, y: 0, width: 100, height: 100 });
    expect(useAnnotateStore.getState().cropConfirmed).toBe(false);
    s.confirmCrop();
    expect(useAnnotateStore.getState().cropConfirmed).toBe(true);
    expect(useAnnotateStore.getState().tool).toBe('select');
  });

  it('setCropRegion is undoable: Ctrl+Z restores the previous (null) region', () => {
    const s = useAnnotateStore.getState();
    s.setCropRegion({ x: 10, y: 20, width: 30, height: 40 });
    expect(useAnnotateStore.getState().cropRegion).toEqual({ x: 10, y: 20, width: 30, height: 40 });
    s.undo();
    expect(useAnnotateStore.getState().cropRegion).toBeNull();
  });

  it('confirmCrop is undoable: Ctrl+Z restores cropConfirmed to false', () => {
    const s = useAnnotateStore.getState();
    s.setCropRegion({ x: 0, y: 0, width: 50, height: 50 });
    s.confirmCrop();
    expect(useAnnotateStore.getState().cropConfirmed).toBe(true);
    s.undo();
    expect(useAnnotateStore.getState().cropConfirmed).toBe(false);
    expect(useAnnotateStore.getState().cropRegion).toEqual({ x: 0, y: 0, width: 50, height: 50 });
  });

  it('cancelling a crop (setCropRegion(null)) is undoable', () => {
    const s = useAnnotateStore.getState();
    s.setCropRegion({ x: 1, y: 2, width: 3, height: 4 });
    s.setCropRegion(null);
    expect(useAnnotateStore.getState().cropRegion).toBeNull();
    s.undo();
    expect(useAnnotateStore.getState().cropRegion).toEqual({ x: 1, y: 2, width: 3, height: 4 });
  });

  it('undo restores shapes and crop state together', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.setCropRegion({ x: 0, y: 0, width: 10, height: 10 });
    s.confirmCrop();
    s.undo();
    expect(useAnnotateStore.getState().cropConfirmed).toBe(false);
    s.undo();
    expect(useAnnotateStore.getState().cropRegion).toBeNull();
    s.undo();
    expect(useAnnotateStore.getState().shapes).toHaveLength(0);
  });

  it('reset wipes shapes, undo/redo stacks, and selection', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.setSelectedId('sh-a');
    s.setTool('rectangle');
    s.reset();
    const after = useAnnotateStore.getState();
    expect(after.shapes).toHaveLength(0);
    expect(after.undoStack).toHaveLength(0);
    expect(after.selectedId).toBeNull();
    expect(after.tool).toBe('select');
  });

  it('sourceImage starts null and round-trips through setSourceImage', () => {
    expect(useAnnotateStore.getState().sourceImage).toBeNull();
    const img = new window.Image();
    useAnnotateStore.getState().setSourceImage(img);
    expect(useAnnotateStore.getState().sourceImage).toBe(img);
    useAnnotateStore.getState().setSourceImage(null);
    expect(useAnnotateStore.getState().sourceImage).toBeNull();
  });

  it('reset clears sourceImage back to null', () => {
    const img = new window.Image();
    useAnnotateStore.getState().setSourceImage(img);
    expect(useAnnotateStore.getState().sourceImage).toBe(img);
    useAnnotateStore.getState().reset();
    expect(useAnnotateStore.getState().sourceImage).toBeNull();
  });

  it('isDirty starts false on a fresh store', () => {
    expect(useAnnotateStore.getState().isDirty).toBe(false);
  });

  it('addShape, updateShape, deleteShape, setCropRegion, confirmCrop all set isDirty', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    expect(useAnnotateStore.getState().isDirty).toBe(false);

    s.updateShape('sh-a', { x: 5 });
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    s.deleteShape('sh-a');
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    s.setCropRegion({ x: 0, y: 0, width: 10, height: 10 });
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    s.confirmCrop();
    expect(useAnnotateStore.getState().isDirty).toBe(true);
  });

  it('markClean preserves shapes, crop, and history', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.setCropRegion({ x: 0, y: 0, width: 50, height: 50 });
    s.markClean();
    const after = useAnnotateStore.getState();
    expect(after.shapes).toHaveLength(1);
    expect(after.cropRegion).toEqual({ x: 0, y: 0, width: 50, height: 50 });
    expect(after.undoStack.length).toBeGreaterThan(0);
    expect(after.isDirty).toBe(false);
  });

  it('reset clears isDirty', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    expect(useAnnotateStore.getState().isDirty).toBe(true);
    s.reset();
    expect(useAnnotateStore.getState().isDirty).toBe(false);
  });
});
