import { useAnnotateStore, COLORS, STROKE_WIDTHS, type StrokePreset } from './store';

import type { AnnotationTool } from '@snk/annotate';

const TOOLS: { id: AnnotationTool; label: string; icon: string }[] = [
  { id: 'arrow', label: 'Arrow', icon: '↗' },
  { id: 'rectangle', label: 'Rectangle', icon: '□' },
  { id: 'ellipse', label: 'Ellipse', icon: '○' },
  { id: 'pen', label: 'Pen', icon: '✎' },
  { id: 'highlighter', label: 'Highlighter', icon: '🖍' },
  { id: 'text', label: 'Text', icon: 'T' },
  { id: 'blur', label: 'Blur', icon: '▦' },
  { id: 'step-marker', label: 'Step', icon: '#' },
  { id: 'crop', label: 'Crop', icon: '⬔' },
];

export function AnnotateToolbar() {
  const tool = useAnnotateStore((s) => s.tool);
  const color = useAnnotateStore((s) => s.color);
  const strokePreset = useAnnotateStore((s) => s.strokePreset);
  const undoStack = useAnnotateStore((s) => s.undoStack);
  const redoStack = useAnnotateStore((s) => s.redoStack);
  const setTool = useAnnotateStore((s) => s.setTool);
  const setColor = useAnnotateStore((s) => s.setColor);
  const setStrokePreset = useAnnotateStore((s) => s.setStrokePreset);
  const undo = useAnnotateStore((s) => s.undo);
  const redo = useAnnotateStore((s) => s.redo);

  return (
    <div className="flex flex-col gap-3 p-2 bg-slate-900 border-r border-slate-700 w-14 items-center">
      <div className="flex flex-col gap-1">
        {TOOLS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTool(t.id)}
            className={`w-10 h-10 rounded flex items-center justify-center text-sm ${
              tool === t.id
                ? 'bg-blue-600 text-white'
                : 'text-slate-400 hover:bg-slate-800'
            }`}
            title={t.label}
          >
            {t.icon}
          </button>
        ))}
      </div>

      <div className="w-8 border-t border-slate-700" />

      <div className="flex flex-col gap-1">
        {COLORS.map((c) => (
          <button
            key={c}
            onClick={() => setColor(c)}
            className={`w-6 h-6 rounded-full border-2 mx-auto ${
              color === c ? 'border-white' : 'border-transparent'
            }`}
            style={{ backgroundColor: c }}
            title={c}
          />
        ))}
      </div>

      <div className="w-8 border-t border-slate-700" />

      <div className="flex flex-col gap-1 items-center">
        {(Object.keys(STROKE_WIDTHS) as StrokePreset[]).map((preset) => (
          <button
            key={preset}
            onClick={() => setStrokePreset(preset)}
            className={`w-10 h-6 rounded flex items-center justify-center ${
              strokePreset === preset
                ? 'bg-blue-600'
                : 'hover:bg-slate-800'
            }`}
            title={preset}
          >
            <div
              className="bg-white rounded-full"
              style={{ width: 20, height: STROKE_WIDTHS[preset] }}
            />
          </button>
        ))}
      </div>

      <div className="w-8 border-t border-slate-700" />

      <div className="flex flex-col gap-1">
        <button
          onClick={undo}
          disabled={undoStack.length === 0}
          className="w-10 h-8 rounded text-sm text-slate-400 hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed"
          title="Undo (Ctrl+Z)"
        >
          ↶
        </button>
        <button
          onClick={redo}
          disabled={redoStack.length === 0}
          className="w-10 h-8 rounded text-sm text-slate-400 hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed"
          title="Redo (Ctrl+Y)"
        >
          ↷
        </button>
      </div>
    </div>
  );
}
