import { useEffect, useRef } from 'react';
import { Rect } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

export function RectangleShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Rect | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Rect
      ref={ref}
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      width={shape.width ?? 0}
      height={shape.height ?? 0}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
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
