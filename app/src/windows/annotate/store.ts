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

// One snapshot per undoable action. Crop draw/confirm/cancel mutate fields
// outside `shapes`, so the history stack has to carry them too.
interface HistoryEntry {
  shapes: AnnotationShape[];
  cropRegion: CropRegion | null;
  cropConfirmed: boolean;
}

interface AnnotateState {
  tool: AnnotationTool;
  color: string;
  strokePreset: StrokePreset;
  shapes: AnnotationShape[];
  undoStack: HistoryEntry[];
  redoStack: HistoryEntry[];
  nextStepNumber: number;
  cropRegion: CropRegion | null;
  cropConfirmed: boolean;
  isDrawing: boolean;
  currentShape: AnnotationShape | null;
  selectedId: string | null;
  sourceImage: HTMLImageElement | null;
  isDirty: boolean;

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
  setSourceImage: (img: HTMLImageElement | null) => void;
  markClean: () => void;
  reset: () => void;
}

const initialState = {
  tool: 'select' as AnnotationTool,
  color: '#ef4444',
  strokePreset: 'medium' as StrokePreset,
  shapes: [] as AnnotationShape[],
  undoStack: [] as HistoryEntry[],
  redoStack: [] as HistoryEntry[],
  nextStepNumber: 1,
  cropRegion: null as CropRegion | null,
  cropConfirmed: false,
  isDrawing: false,
  currentShape: null as AnnotationShape | null,
  selectedId: null as string | null,
  sourceImage: null as HTMLImageElement | null,
  isDirty: false,
};

function snapshot(s: {
  shapes: AnnotationShape[];
  cropRegion: CropRegion | null;
  cropConfirmed: boolean;
}): HistoryEntry {
  return {
    shapes: s.shapes,
    cropRegion: s.cropRegion,
    cropConfirmed: s.cropConfirmed,
  };
}

export const useAnnotateStore = create<AnnotateState>((set, get) => ({
  ...initialState,

  setTool: (tool) => set({ tool, selectedId: tool === 'select' ? get().selectedId : null }),
  setColor: (color) => set({ color }),
  setStrokePreset: (preset) => set({ strokePreset: preset }),

  addShape: (shape) => {
    const state = get();
    set({
      undoStack: [...state.undoStack, snapshot(state)],
      redoStack: [],
      shapes: [...state.shapes, shape],
      nextStepNumber:
        shape.tool === 'step-marker' ? state.nextStepNumber + 1 : state.nextStepNumber,
      isDirty: true,
    });
  },

  updateShape: (id, patch) => {
    const state = get();
    set({
      undoStack: [...state.undoStack, snapshot(state)],
      redoStack: [],
      shapes: state.shapes.map((s) => (s.id === id ? { ...s, ...patch } : s)),
      isDirty: true,
    });
  },

  deleteShape: (id) => {
    const state = get();
    set({
      undoStack: [...state.undoStack, snapshot(state)],
      redoStack: [],
      shapes: state.shapes.filter((s) => s.id !== id),
      selectedId: state.selectedId === id ? null : state.selectedId,
      isDirty: true,
    });
  },

  // undo/redo intentionally do NOT touch isDirty. Restoring from history
  // is not a 'change' in the user's mental model — and tracking the
  // precise diff against the last saved state isn't worth the
  // complexity. A user who edits then undoes back to baseline will
  // still see the dirty prompt on close; that's an accepted
  // trade-off.
  undo: () => {
    const state = get();
    if (state.undoStack.length === 0) return;
    const prev = state.undoStack[state.undoStack.length - 1]!;
    set({
      undoStack: state.undoStack.slice(0, -1),
      redoStack: [...state.redoStack, snapshot(state)],
      shapes: prev.shapes,
      cropRegion: prev.cropRegion,
      cropConfirmed: prev.cropConfirmed,
    });
  },

  redo: () => {
    const state = get();
    if (state.redoStack.length === 0) return;
    const next = state.redoStack[state.redoStack.length - 1]!;
    set({
      redoStack: state.redoStack.slice(0, -1),
      undoStack: [...state.undoStack, snapshot(state)],
      shapes: next.shapes,
      cropRegion: next.cropRegion,
      cropConfirmed: next.cropConfirmed,
    });
  },

  // Drawing a fresh region resets the confirmation; the user must
  // explicitly Apply it before it counts at export time. Snapshots the
  // pre-mutation state so Ctrl+Z restores it.
  setCropRegion: (region) => {
    const state = get();
    set({
      undoStack: [...state.undoStack, snapshot(state)],
      redoStack: [],
      cropRegion: region,
      cropConfirmed: false,
      isDirty: true,
    });
  },
  confirmCrop: () => {
    const state = get();
    set({
      undoStack: [...state.undoStack, snapshot(state)],
      redoStack: [],
      cropConfirmed: true,
      tool: 'select',
      isDirty: true,
    });
  },
  setIsDrawing: (drawing) => set({ isDrawing: drawing }),
  setCurrentShape: (shape) => set({ currentShape: shape }),
  setSelectedId: (id) => set({ selectedId: id }),
  setSourceImage: (img) => set({ sourceImage: img }),
  markClean: () => set({ isDirty: false }),
  reset: () => set(initialState),
}));
