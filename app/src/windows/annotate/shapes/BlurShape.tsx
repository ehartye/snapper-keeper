import { useEffect, useRef } from 'react';
import { Image as KonvaImage } from 'react-konva';
import Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

// 20px gives macOS-style chunky pixelation. Large enough that even
// uppercase letters lose their identifiable shape entirely; small
// enough to still feel like a "redaction box" rather than a giant blob.
const PIXELATE_BLOCK_SIZE = 20;

export function BlurShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Image | null>(null);
  const sourceImage = useAnnotateStore((s) => s.sourceImage);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  const x = shape.x ?? 0;
  const y = shape.y ?? 0;
  const w = shape.width ?? 0;
  const h = shape.height ?? 0;

  // Konva filters only render against a cached node. Re-cache whenever
  // the source image or the shape geometry changes so the filter
  // re-runs against the right region.
  useEffect(() => {
    const node = ref.current;
    if (!node || !sourceImage || w <= 0 || h <= 0) return;
    node.cache();
    node.getLayer()?.batchDraw();
  }, [sourceImage, x, y, w, h]);

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <KonvaImage
      ref={ref}
      image={sourceImage ?? undefined}
      x={x}
      y={y}
      width={w}
      height={h}
      // The screenshot fills the layer at (0,0) at native pixel size,
      // so the shape's rect and the crop region happen to be identical.
      crop={{ x, y, width: w, height: h }}
      filters={[Konva.Filters.Pixelate]}
      pixelSize={PIXELATE_BLOCK_SIZE}
      draggable={draggable}
      onMouseDown={onSelect}
      onTap={onSelect}
      onDragEnd={(e) => {
        updateShape(shape.id, { x: e.target.x(), y: e.target.y() });
      }}
      onTransformEnd={() => {
        const node = ref.current;
        if (!node) return;
        const scaleX = node.scaleX();
        const scaleY = node.scaleY();
        node.scaleX(1);
        node.scaleY(1);
        updateShape(shape.id, {
          x: node.x(),
          y: node.y(),
          width: Math.max(5, node.width() * scaleX),
          height: Math.max(5, node.height() * scaleY),
        });
      }}
    />
  );
}
