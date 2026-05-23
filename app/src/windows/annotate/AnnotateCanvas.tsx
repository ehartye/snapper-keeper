import { useRef, useEffect, useState, useCallback, type MutableRefObject } from 'react';
import { Stage, Layer, Image as KonvaImage, Rect, Transformer } from 'react-konva';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type Konva from 'konva';

import { useAnnotateStore } from './store';
import { confirmDiscardIfDirty } from './dirtyGuard';
import { ShapeRenderer } from './shapes';
import { useDrawing } from './useDrawing';

interface Props {
  imageSrc: string;
  imageWidth: number;
  imageHeight: number;
  stageRef: MutableRefObject<Konva.Stage | null>;
}

interface EditingText {
  id: string;
  value: string;
  x: number;
  y: number;
  fontSize: number;
  color: string;
}

export function AnnotateCanvas({ imageSrc, imageWidth, imageHeight, stageRef }: Props) {
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [containerSize, setContainerSize] = useState({ width: 800, height: 600 });
  const [editing, setEditing] = useState<EditingText | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const transformerRef = useRef<Konva.Transformer | null>(null);
  const nodesRef = useRef<Map<string, Konva.Node>>(new Map());
  const cropOutlineRef = useRef<Konva.Rect | null>(null);
  const cropTransformerRef = useRef<Konva.Transformer | null>(null);

  const tool = useAnnotateStore((s) => s.tool);
  const shapes = useAnnotateStore((s) => s.shapes);
  const currentShape = useAnnotateStore((s) => s.currentShape);
  const cropRegion = useAnnotateStore((s) => s.cropRegion);
  const cropConfirmed = useAnnotateStore((s) => s.cropConfirmed);
  const selectedId = useAnnotateStore((s) => s.selectedId);
  const setSelectedId = useAnnotateStore((s) => s.setSelectedId);
  const setCropRegion = useAnnotateStore((s) => s.setCropRegion);
  const resizeCropRegion = useAnnotateStore((s) => s.resizeCropRegion);
  const setSourceImage = useAnnotateStore((s) => s.setSourceImage);
  const confirmCrop = useAnnotateStore((s) => s.confirmCrop);
  const deleteShape = useAnnotateStore((s) => s.deleteShape);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  const { handleMouseDown, handleMouseMove, handleMouseUp } = useDrawing(stageRef);

  useEffect(() => {
    let cancelled = false;
    const img = new window.Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      if (cancelled) return;
      setImage(img);
      setSourceImage(img);
    };
    img.src = imageSrc;
    return () => {
      // reset() also clears sourceImage; this handles the imageSrc-change case.
      cancelled = true;
      setSourceImage(null);
    };
  }, [imageSrc, setSourceImage]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        setContainerSize({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        });
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const scale = Math.min(
    containerSize.width / imageWidth,
    containerSize.height / imageHeight,
    1,
  );

  const stageWidth = imageWidth * scale;
  const stageHeight = imageHeight * scale;

  const selectedShape = shapes.find((s) => s.id === selectedId) ?? null;

  useEffect(() => {
    const transformer = transformerRef.current;
    if (!transformer) return;
    if (selectedId) {
      const node = nodesRef.current.get(selectedId);
      if (node) {
        transformer.nodes([node]);
      } else {
        transformer.nodes([]);
      }
    } else {
      transformer.nodes([]);
    }
    transformer.getLayer()?.batchDraw();
  }, [selectedId, shapes]);

  // Attach the crop transformer to the outline whenever both exist —
  // the outline is the (offset-by-half-stroke) rect we draw on top of
  // the dim mask. The user grabs handles on this rect; resize/drag
  // commits the actual crop bounds via resizeCropRegion (preserving
  // cropConfirmed so an Apply'd crop stays applied through resizes).
  useEffect(() => {
    const transformer = cropTransformerRef.current;
    const outline = cropOutlineRef.current;
    if (!transformer) return;
    if (outline && cropRegion) {
      transformer.nodes([outline]);
    } else {
      transformer.nodes([]);
    }
    transformer.getLayer()?.batchDraw();
  }, [cropRegion, cropConfirmed, currentShape]);

  const transformableTools = new Set([
    'rectangle',
    'ellipse',
    'blur',
    'text',
    'arrow',
    'pen',
    'highlighter',
    'step-marker',
  ]);
  const canTransformSelected =
    selectedShape !== null && transformableTools.has(selectedShape.tool);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (editing) return;

      if (e.key === 'Escape') {
        if (selectedId) {
          setSelectedId(null);
          return;
        }
        if (!confirmDiscardIfDirty()) return;
        useAnnotateStore.getState().reset();
        getCurrentWindow().hide();
        return;
      }
      if ((e.key === 'Delete' || e.key === 'Backspace') && selectedId) {
        const target = e.target as HTMLElement | null;
        if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;
        e.preventDefault();
        deleteShape(selectedId);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          useAnnotateStore.getState().redo();
        } else {
          useAnnotateStore.getState().undo();
        }
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
        e.preventDefault();
        useAnnotateStore.getState().redo();
      }
    },
    [editing, selectedId, setSelectedId, deleteShape],
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  const startEditingSelectedText = useCallback(() => {
    if (!selectedShape || selectedShape.tool !== 'text') return;
    setEditing({
      id: selectedShape.id,
      value: selectedShape.text ?? '',
      x: selectedShape.x ?? 0,
      y: selectedShape.y ?? 0,
      fontSize: selectedShape.stroke.width * 6,
      color: selectedShape.stroke.color,
    });
  }, [selectedShape]);

  const commitEdit = useCallback(() => {
    if (!editing) return;
    const trimmed = editing.value;
    if (trimmed.length === 0) {
      deleteShape(editing.id);
    } else {
      updateShape(editing.id, { text: trimmed });
    }
    setEditing(null);
  }, [editing, updateShape, deleteShape]);

  const handleStageMouseDown = useCallback(
    (evt: Konva.KonvaEventObject<MouseEvent>) => {
      if (tool === 'select') {
        const stage = stageRef.current;
        if (evt.target === stage) {
          setSelectedId(null);
        }
        return;
      }
      handleMouseDown(evt);
    },
    [tool, stageRef, setSelectedId, handleMouseDown],
  );

  const registerNode = useCallback(
    (id: string) => (node: unknown | null) => {
      if (node) {
        nodesRef.current.set(id, node as Konva.Node);
      } else {
        nodesRef.current.delete(id);
      }
    },
    [],
  );

  // Position of the text edit overlay in screen pixels (scaled by stage scale)
  const editScreenStyle = editing
    ? {
        left: editing.x * scale,
        top: editing.y * scale,
        fontSize: editing.fontSize * scale,
        color: editing.color,
      }
    : undefined;

  return (
    <div
      ref={containerRef}
      className="flex-1 overflow-hidden bg-bg-soft flex items-center justify-center"
    >
      <div className="relative" style={{ width: stageWidth, height: stageHeight }}>
        <Stage
          ref={stageRef}
          width={stageWidth}
          height={stageHeight}
          scaleX={scale}
          scaleY={scale}
          onMouseDown={handleStageMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
        >
          <Layer listening={false}>
            {image && <KonvaImage image={image} width={imageWidth} height={imageHeight} />}
          </Layer>
          <Layer>
            {shapes.map((s) => (
              <ShapeRenderer
                key={s.id}
                shape={s}
                draggable={tool === 'select'}
                onSelect={() => {
                  if (tool === 'select') setSelectedId(s.id);
                }}
                registerNode={registerNode(s.id)}
                onStartEditText={
                  s.tool === 'text' && tool === 'select'
                    ? () => {
                        setSelectedId(s.id);
                        setEditing({
                          id: s.id,
                          value: s.text ?? '',
                          x: s.x ?? 0,
                          y: s.y ?? 0,
                          fontSize: s.stroke.width * 6,
                          color: s.stroke.color,
                        });
                      }
                    : undefined
                }
              />
            ))}
            {currentShape && currentShape.tool === 'blur' ? (
              // While the user is dragging out a new blur rect, draw a
              // cheap dashed Rect instead of running Pixelate + cache on
              // every mousemove. The real BlurShape only renders for
              // committed shapes in the `shapes` array.
              <Rect
                x={currentShape.x ?? 0}
                y={currentShape.y ?? 0}
                width={currentShape.width ?? 0}
                height={currentShape.height ?? 0}
                stroke="#3b82f6"
                strokeWidth={2}
                dash={[6, 3]}
                fill="rgba(59, 130, 246, 0.15)"
              />
            ) : currentShape ? (
              <ShapeRenderer shape={currentShape} />
            ) : null}
            {tool === 'select' && canTransformSelected && (
              <Transformer
                ref={transformerRef}
                rotateEnabled={false}
                boundBoxFunc={(oldBox, newBox) => {
                  if (newBox.width < 5 || newBox.height < 5) return oldBox;
                  return newBox;
                }}
              />
            )}
          </Layer>
          {(() => {
            // Render the crop preview from either the in-progress draft
            // (currentShape, during the drag) or the committed cropRegion
            // (post-mouseup, pre-Apply, or confirmed). Solid green only
            // for a confirmed crop; dashed blue otherwise.
            const drafting = currentShape?.tool === 'crop';
            const preview = drafting
              ? {
                  x: currentShape.x ?? 0,
                  y: currentShape.y ?? 0,
                  width: currentShape.width ?? 0,
                  height: currentShape.height ?? 0,
                }
              : cropRegion;
            if (!preview || preview.width <= 0 || preview.height <= 0) return null;
            const confirmed = !drafting && cropConfirmed;
            const strokeW = confirmed ? 3 : 2;
            const half = strokeW / 2;
            return (
              // Layer must listen so the outline's drag/transform events
              // fire. Each dim rect is individually marked listening=false
              // so they don't intercept clicks on the shape layer below.
              <Layer>
                {/* Dim everything outside the previewed crop so the user
                    can see what will be kept vs. cropped away. Export uses
                    the same crop bounds, so these dimmed pixels never
                    appear in the output file. */}
                <Rect
                  listening={false}
                  x={0}
                  y={0}
                  width={imageWidth}
                  height={preview.y}
                  fill="rgba(0, 0, 0, 0.45)"
                />
                <Rect
                  listening={false}
                  x={0}
                  y={preview.y + preview.height}
                  width={imageWidth}
                  height={Math.max(0, imageHeight - (preview.y + preview.height))}
                  fill="rgba(0, 0, 0, 0.45)"
                />
                <Rect
                  listening={false}
                  x={0}
                  y={preview.y}
                  width={preview.x}
                  height={preview.height}
                  fill="rgba(0, 0, 0, 0.45)"
                />
                <Rect
                  listening={false}
                  x={preview.x + preview.width}
                  y={preview.y}
                  width={Math.max(0, imageWidth - (preview.x + preview.width))}
                  height={preview.height}
                  fill="rgba(0, 0, 0, 0.45)"
                />
                {/* Konva centers stroke on the path; we shift the outline
                    outward by half-stroke so the stroke sits entirely
                    outside the kept region (otherwise toDataURL bakes a
                    thin green border into saved derivations). */}
                <Rect
                  ref={cropOutlineRef}
                  x={preview.x - half}
                  y={preview.y - half}
                  width={preview.width + strokeW}
                  height={preview.height + strokeW}
                  stroke={confirmed ? '#22c55e' : '#3b82f6'}
                  strokeWidth={strokeW}
                  dash={confirmed ? undefined : [6, 3]}
                  draggable={!drafting}
                  onDragEnd={(e) => {
                    // Un-shift the half-stroke offset to recover the
                    // actual kept-region bounds.
                    resizeCropRegion({
                      x: e.target.x() + half,
                      y: e.target.y() + half,
                      width: preview.width,
                      height: preview.height,
                    });
                  }}
                  onTransformEnd={() => {
                    const node = cropOutlineRef.current;
                    if (!node) return;
                    const sx = node.scaleX();
                    const sy = node.scaleY();
                    const newOutW = node.width() * sx;
                    const newOutH = node.height() * sy;
                    const newOutX = node.x();
                    const newOutY = node.y();
                    node.scaleX(1);
                    node.scaleY(1);
                    resizeCropRegion({
                      x: newOutX + half,
                      y: newOutY + half,
                      width: Math.max(5, newOutW - strokeW),
                      height: Math.max(5, newOutH - strokeW),
                    });
                  }}
                />
                {!drafting && (
                  <Transformer
                    ref={cropTransformerRef}
                    rotateEnabled={false}
                    boundBoxFunc={(oldBox, newBox) => {
                      if (newBox.width < 5 + strokeW || newBox.height < 5 + strokeW) {
                        return oldBox;
                      }
                      return newBox;
                    }}
                  />
                )}
              </Layer>
            );
          })()}
        </Stage>

        {editing && editScreenStyle && (
          <textarea
            autoFocus
            value={editing.value}
            onChange={(e) => setEditing({ ...editing, value: e.target.value })}
            onBlur={commitEdit}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                commitEdit();
              }
              if (e.key === 'Escape') {
                e.preventDefault();
                setEditing(null);
              }
              e.stopPropagation();
            }}
            className="absolute bg-transparent border border-blue-400 outline-none resize-none p-0 m-0 leading-tight"
            style={{
              ...editScreenStyle,
              fontFamily: 'system-ui, sans-serif',
              minWidth: 40,
            }}
            // Resize the textarea to fit content as the user types.
            rows={Math.max(1, editing.value.split('\n').length)}
          />
        )}

        {selectedShape && tool === 'select' && selectedShape.tool === 'text' && (
          <button
            className="absolute text-xs bg-slate-800 text-slate-200 px-2 py-0.5 rounded shadow"
            style={{
              left: (selectedShape.x ?? 0) * scale,
              top: ((selectedShape.y ?? 0) + selectedShape.stroke.width * 6 + 4) * scale,
            }}
            onClick={startEditingSelectedText}
          >
            edit
          </button>
        )}

        {cropRegion && !cropConfirmed && (
          <div
            className="absolute flex gap-1.5 pointer-events-auto"
            style={{
              left: (cropRegion.x + cropRegion.width) * scale - 110,
              top: (cropRegion.y + cropRegion.height) * scale + 6,
            }}
          >
            <button
              onClick={() => setCropRegion(null)}
              className="font-display text-[10px] uppercase tracking-wider px-3 py-1.5 bg-surface text-fg border-2 border-border shadow-[2px_2px_0_0_var(--border)] hover:translate-x-0.5 hover:translate-y-0.5 hover:shadow-[0_0_0_0_var(--border)] transition-transform"
            >
              cancel
            </button>
            <button
              onClick={confirmCrop}
              className="font-display text-[10px] uppercase tracking-wider px-3 py-1.5 bg-accent-3 text-bg border-2 border-border shadow-[2px_2px_0_0_var(--border)] hover:translate-x-0.5 hover:translate-y-0.5 hover:shadow-[0_0_0_0_var(--border)] transition-transform"
            >
              apply ✓
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
