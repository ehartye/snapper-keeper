import { useRef, useEffect, useState, useCallback, type MutableRefObject } from 'react';
import { Stage, Layer, Image as KonvaImage, Rect } from 'react-konva';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type Konva from 'konva';

import { useAnnotateStore } from './store';
import { ShapeRenderer } from './shapes';
import { useDrawing } from './useDrawing';

interface Props {
  imageSrc: string;
  imageWidth: number;
  imageHeight: number;
  stageRef: MutableRefObject<Konva.Stage | null>;
}

export function AnnotateCanvas({ imageSrc, imageWidth, imageHeight, stageRef }: Props) {
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [containerSize, setContainerSize] = useState({ width: 800, height: 600 });
  const containerRef = useRef<HTMLDivElement>(null);

  const shapes = useAnnotateStore((s) => s.shapes);
  const currentShape = useAnnotateStore((s) => s.currentShape);
  const cropRegion = useAnnotateStore((s) => s.cropRegion);

  const { handleMouseDown, handleMouseMove, handleMouseUp } = useDrawing(stageRef);

  useEffect(() => {
    const img = new window.Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => setImage(img);
    img.src = imageSrc;
  }, [imageSrc]);

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

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        useAnnotateStore.getState().reset();
        getCurrentWindow().hide();
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
    [],
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div ref={containerRef} className="flex-1 overflow-hidden bg-slate-950 flex items-center justify-center">
      <Stage
        ref={stageRef}
        width={stageWidth}
        height={stageHeight}
        scaleX={scale}
        scaleY={scale}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
      >
        <Layer>
          {image && (
            <KonvaImage image={image} width={imageWidth} height={imageHeight} />
          )}
        </Layer>
        <Layer>
          {shapes.map((s) => (
            <ShapeRenderer key={s.id} shape={s} />
          ))}
          {currentShape && <ShapeRenderer shape={currentShape} />}
        </Layer>
        {cropRegion && (
          <Layer>
            <Rect
              x={cropRegion.x}
              y={cropRegion.y}
              width={cropRegion.width}
              height={cropRegion.height}
              stroke="#3b82f6"
              strokeWidth={2}
              dash={[6, 3]}
            />
          </Layer>
        )}
      </Stage>
    </div>
  );
}
