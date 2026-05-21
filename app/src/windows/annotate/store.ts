import { create } from 'zustand';

import type { AnnotationTool, AnnotationShape } from '@snk/annotate';

export const COLORS = [
  '#ef4444',
  '#f97316',
  '#eab308',
  '#22c55e',
  '#3b82f6',
  '#8b5cf6',
  '#000000',
  '#ffffff',
] as const;

export const STROKE_WIDTHS = {
  thin: 2,
  medium: 4,
  thick: 8,
} as const;

export type StrokePreset = keyof typeof STROKE_WIDTHS;

interface CropRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface AnnotateState {
  tool: AnnotationTool;
  color: string;
  strokePreset: StrokePreset;
  shapes: AnnotationShape[];
  undoStack: AnnotationShape[][];
  redoStack: AnnotationShape[][];
  nextStepNumber: number;
  cropRegion: CropRegion | null;
  isDrawing: boolean;
  currentShape: AnnotationShape | null;

  setTool: (tool: AnnotationTool) => void;
  setColor: (color: string) => void;
  setStrokePreset: (preset: StrokePreset) => void;
  addShape: (shape: AnnotationShape) => void;
  undo: () => void;
  redo: () => void;
  setCropRegion: (region: CropRegion | null) => void;
  setIsDrawing: (drawing: boolean) => void;
  setCurrentShape: (shape: AnnotationShape | null) => void;
  reset: () => void;
}

const initialState = {
  tool: 'arrow' as AnnotationTool,
  color: '#ef4444',
  strokePreset: 'medium' as StrokePreset,
  shapes: [] as AnnotationShape[],
  undoStack: [] as AnnotationShape[][],
  redoStack: [] as AnnotationShape[][],
  nextStepNumber: 1,
  cropRegion: null as CropRegion | null,
  isDrawing: false,
  currentShape: null as AnnotationShape | null,
};

export const useAnnotateStore = create<AnnotateState>((set, get) => ({
  ...initialState,

  setTool: (tool) => set({ tool }),
  setColor: (color) => set({ color }),
  setStrokePreset: (preset) => set({ strokePreset: preset }),

  addShape: (shape) => {
    const { shapes, nextStepNumber } = get();
    set({
      undoStack: [...get().undoStack, shapes],
      redoStack: [],
      shapes: [...shapes, shape],
      nextStepNumber: shape.tool === 'step-marker' ? nextStepNumber + 1 : nextStepNumber,
    });
  },

  undo: () => {
    const { undoStack, shapes } = get();
    if (undoStack.length === 0) return;
    const prev = undoStack[undoStack.length - 1]!;
    set({
      undoStack: undoStack.slice(0, -1),
      redoStack: [...get().redoStack, shapes],
      shapes: prev,
    });
  },

  redo: () => {
    const { redoStack, shapes } = get();
    if (redoStack.length === 0) return;
    const next = redoStack[redoStack.length - 1]!;
    set({
      redoStack: redoStack.slice(0, -1),
      undoStack: [...get().undoStack, shapes],
      shapes: next,
    });
  },

  setCropRegion: (region) => set({ cropRegion: region }),
  setIsDrawing: (drawing) => set({ isDrawing: drawing }),
  setCurrentShape: (shape) => set({ currentShape: shape }),
  reset: () => set(initialState),
}));
