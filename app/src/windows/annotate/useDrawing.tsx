import { useCallback, useRef, type RefObject } from 'react';
import type Konva from 'konva';

import type { AnnotationShape, AnnotationTool } from '@snk/annotate';

import { useAnnotateStore, STROKE_WIDTHS } from './store';

let shapeCounter = 0;
function nextId(): string {
  shapeCounter += 1;
  return `shape-${shapeCounter}`;
}

function makeStroke(color: string, width: number, tool: AnnotationTool) {
  return {
    color,
    width,
    opacity: tool === 'highlighter' ? 0.35 : 1,
  };
}

export function useDrawing(stageRef: RefObject<Konva.Stage | null>) {
  const startPosRef = useRef<{ x: number; y: number } | null>(null);

  const getPointerPos = useCallback(() => {
    const stage = stageRef.current;
    if (!stage) return null;
    return stage.getPointerPosition();
  }, [stageRef]);

  const handleMouseDown = useCallback(() => {
    const pos = getPointerPos();
    if (!pos) return;

    const { tool, color, strokePreset, nextStepNumber } = useAnnotateStore.getState();
    const strokeWidth = STROKE_WIDTHS[strokePreset];
    const stroke = makeStroke(color, strokeWidth, tool);

    startPosRef.current = pos;

    if (tool === 'step-marker') {
      useAnnotateStore.getState().addShape({
        id: nextId(),
        tool,
        x: pos.x,
        y: pos.y,
        stroke,
        stepNumber: nextStepNumber,
      });
      return;
    }

    if (tool === 'text') {
      const text = window.prompt('Enter text:');
      if (text) {
        useAnnotateStore.getState().addShape({
          id: nextId(),
          tool,
          x: pos.x,
          y: pos.y,
          text,
          stroke,
        });
      }
      return;
    }

    const shape: AnnotationShape = {
      id: nextId(),
      tool,
      stroke,
      ...(tool === 'arrow' || tool === 'pen' || tool === 'highlighter'
        ? { points: [pos.x, pos.y] }
        : { x: pos.x, y: pos.y, width: 0, height: 0 }),
    };

    useAnnotateStore.getState().setCurrentShape(shape);
    useAnnotateStore.getState().setIsDrawing(true);
  }, [getPointerPos]);

  const handleMouseMove = useCallback(() => {
    const { isDrawing, currentShape } = useAnnotateStore.getState();
    if (!isDrawing || !currentShape) return;

    const pos = getPointerPos();
    if (!pos) return;

    const start = startPosRef.current;
    if (!start) return;

    const tool = currentShape.tool;

    if (tool === 'pen' || tool === 'highlighter') {
      useAnnotateStore.getState().setCurrentShape({
        ...currentShape,
        points: [...(currentShape.points ?? []), pos.x, pos.y],
      });
      return;
    }

    if (tool === 'arrow') {
      useAnnotateStore.getState().setCurrentShape({
        ...currentShape,
        points: [start.x, start.y, pos.x, pos.y],
      });
      return;
    }

    useAnnotateStore.getState().setCurrentShape({
      ...currentShape,
      x: Math.min(start.x, pos.x),
      y: Math.min(start.y, pos.y),
      width: Math.abs(pos.x - start.x),
      height: Math.abs(pos.y - start.y),
    });
  }, [getPointerPos]);

  const handleMouseUp = useCallback(() => {
    const { isDrawing, currentShape, tool } = useAnnotateStore.getState();
    if (!isDrawing || !currentShape) return;

    useAnnotateStore.getState().setIsDrawing(false);
    useAnnotateStore.getState().setCurrentShape(null);
    startPosRef.current = null;

    if (tool === 'crop') {
      useAnnotateStore.getState().setCropRegion({
        x: currentShape.x ?? 0,
        y: currentShape.y ?? 0,
        width: currentShape.width ?? 0,
        height: currentShape.height ?? 0,
      });
      return;
    }

    useAnnotateStore.getState().addShape(currentShape);
  }, []);

  return { handleMouseDown, handleMouseMove, handleMouseUp };
}
