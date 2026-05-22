import { useEffect, useRef } from 'react';
import { Rect, Group } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

export function BlurShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Group | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  const w = shape.width ?? 0;
  const h = shape.height ?? 0;
  const blockSize = 12;
  const cols = Math.ceil(w / blockSize);
  const rows = Math.ceil(h / blockSize);
  const blocks: { bx: number; by: number; shade: string }[] = [];

  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const shade = (r + c) % 2 === 0 ? 'rgba(100,100,100,0.85)' : 'rgba(140,140,140,0.85)';
      blocks.push({ bx: c * blockSize, by: r * blockSize, shade });
    }
  }

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Group
      ref={ref}
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      width={w}
      height={h}
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
          width: Math.max(5, w * scaleX),
          height: Math.max(5, h * scaleY),
        });
      }}
    >
      {blocks.map((b, i) => (
        <Rect
          key={i}
          x={b.bx}
          y={b.by}
          width={Math.min(blockSize, w - b.bx)}
          height={Math.min(blockSize, h - b.by)}
          fill={b.shade}
        />
      ))}
    </Group>
  );
}
