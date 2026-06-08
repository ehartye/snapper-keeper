import { useEffect, useRef, useState } from 'react';

import { useAnnotateStore, COLORS, STROKE_WIDTHS, type StrokePreset } from './store';
import { formatShortcutForPlatform } from '../../lib/shortcuts';

import type { AnnotationTool } from '@snk/annotate';

const TOOLS: { id: AnnotationTool; label: string; icon: string }[] = [
  { id: 'select', label: 'Select', icon: '↖' },
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

  const [colorPickerOpen, setColorPickerOpen] = useState(false);
  const colorRowRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!colorPickerOpen) return;
    const close = (e: MouseEvent) => {
      if (colorRowRef.current && !colorRowRef.current.contains(e.target as Node)) {
        setColorPickerOpen(false);
      }
    };
    document.addEventListener('mousedown', close);
    return () => document.removeEventListener('mousedown', close);
  }, [colorPickerOpen]);

  return (
    <div className="flex flex-col gap-3 p-2 bg-bg-soft border-r border-border w-14 items-center">
      <div className="flex flex-col gap-1">
        {TOOLS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTool(t.id)}
            className={`w-10 h-10 rounded-lg flex items-center justify-center text-sm transition-colors ${
              tool === t.id
                ? 'bg-primary text-bg'
                : 'text-fg-muted hover:bg-surface-2 hover:text-fg'
            }`}
            title={t.label}
          >
            {t.icon}
          </button>
        ))}
      </div>

      <div className="w-8 border-t border-border" />

      <div ref={colorRowRef} className="relative">
        <button
          onClick={() => setColorPickerOpen((o) => !o)}
          className="w-8 h-8 rounded-full border-2 border-fg shadow-[2px_2px_0_0_var(--border)]"
          style={{ backgroundColor: color }}
          title="Color"
        />
        {colorPickerOpen && (
          <div className="absolute left-full top-1/2 -translate-y-1/2 ml-3 bg-surface border-2 border-border rounded-xl shadow-[4px_4px_0_0_var(--accent-2)] p-2 z-50 w-max">
            <div className="grid grid-cols-4 gap-2 w-max">
              {COLORS.map((c) => (
                <button
                  key={c}
                  onClick={() => {
                    setColor(c);
                    setColorPickerOpen(false);
                  }}
                  className={`w-7 h-7 rounded-full border-2 ${
                    color === c ? 'border-fg' : 'border-transparent hover:border-fg-muted'
                  }`}
                  style={{ backgroundColor: c }}
                  title={c}
                />
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="w-8 border-t border-border" />

      <div className="flex flex-col gap-1 items-center">
        {(Object.keys(STROKE_WIDTHS) as StrokePreset[]).map((preset) => (
          <button
            key={preset}
            onClick={() => setStrokePreset(preset)}
            className={`w-10 h-6 rounded-lg flex items-center justify-center ${
              strokePreset === preset ? 'bg-primary' : 'hover:bg-surface-2'
            }`}
            title={preset}
          >
            <div
              className={`rounded-full ${strokePreset === preset ? 'bg-bg' : 'bg-fg'}`}
              style={{ width: 20, height: STROKE_WIDTHS[preset] }}
            />
          </button>
        ))}
      </div>

      <div className="w-8 border-t border-border" />

      <div className="flex flex-col gap-1">
        <button
          onClick={undo}
          disabled={undoStack.length === 0}
          className="w-10 h-8 rounded-lg text-sm text-fg-muted hover:bg-surface-2 hover:text-fg disabled:opacity-30 disabled:cursor-not-allowed"
          title={`Undo (${formatShortcutForPlatform('Ctrl+Z')})`}
        >
          ↶
        </button>
        <button
          onClick={redo}
          disabled={redoStack.length === 0}
          className="w-10 h-8 rounded-lg text-sm text-fg-muted hover:bg-surface-2 hover:text-fg disabled:opacity-30 disabled:cursor-not-allowed"
          title={`Redo (${formatShortcutForPlatform('Ctrl+Y')})`}
        >
          ↷
        </button>
      </div>
    </div>
  );
}
