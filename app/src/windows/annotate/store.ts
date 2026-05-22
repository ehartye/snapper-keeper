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
  cropConfirmed: boolean;
  isDrawing: boolean;
  currentShape: AnnotationShape | null;
  selectedId: string | null;

  setTool: (tool: AnnotationTool) => void;
  setColor: (color: string) => void;
  setStrokePreset: (preset: StrokePreset) => void;
  addShape: (shape: AnnotationShape) => void;
  updateShape: (id: string, patch: Partial<AnnotationShape>) => void;
  deleteShape: (id: string) => void;
  undo: () => void;
  redo: () => void;
  setCropRegion: (region: CropRegion | null) => void;
  confirmCrop: () => void;
  setIsDrawing: (drawing: boolean) => void;
  setCurrentShape: (shape: AnnotationShape | null) => void;
  setSelectedId: (id: string | null) => void;
  reset: () => void;
}

const initialState = {
  tool: 'select' as AnnotationTool,
  color: '#ef4444',
  strokePreset: 'medium' as StrokePreset,
  shapes: [] as AnnotationShape[],
  undoStack: [] as AnnotationShape[][],
  redoStack: [] as AnnotationShape[][],
  nextStepNumber: 1,
  cropRegion: null as CropRegion | null,
  cropConfirmed: false,
  isDrawing: false,
  currentShape: null as AnnotationShape | null,
  selectedId: null as string | null,
};

export const useAnnotateStore = create<AnnotateState>((set, get) => ({
  ...initialState,

  setTool: (tool) => set({ tool, selectedId: tool === 'select' ? get().selectedId : null }),
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

  updateShape: (id, patch) => {
    const { shapes } = get();
    set({
      undoStack: [...get().undoStack, shapes],
      redoStack: [],
      shapes: shapes.map((s) => (s.id === id ? { ...s, ...patch } : s)),
    });
  },

  deleteShape: (id) => {
    const { shapes, selectedId } = get();
    set({
      undoStack: [...get().undoStack, shapes],
      redoStack: [],
      shapes: shapes.filter((s) => s.id !== id),
      selectedId: selectedId === id ? null : selectedId,
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

  // Drawing a fresh region resets the confirmation; the user must
  // explicitly Apply it before it counts at export time.
  setCropRegion: (region) => set({ cropRegion: region, cropConfirmed: false }),
  confirmCrop: () => set({ cropConfirmed: true, tool: 'select' }),
  setIsDrawing: (drawing) => set({ isDrawing: drawing }),
  setCurrentShape: (shape) => set({ currentShape: shape }),
  setSelectedId: (id) => set({ selectedId: id }),
  reset: () => set(initialState),
}));
