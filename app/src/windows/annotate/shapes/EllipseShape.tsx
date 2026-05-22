import { useEffect, useRef } from 'react';
import { Ellipse } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

export function EllipseShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Ellipse | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  const w = shape.width ?? 0;
  const h = shape.height ?? 0;
  const rx = Math.abs(w / 2);
  const ry = Math.abs(h / 2);

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Ellipse
      ref={ref}
      x={(shape.x ?? 0) + rx}
      y={(shape.y ?? 0) + ry}
      radiusX={rx}
      radiusY={ry}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
      draggable={draggable}
      onMouseDown={onSelect}
      onTap={onSelect}
      onDragEnd={(e) => {
        // Konva ellipse positioned at center; convert back to top-left x/y
        updateShape(shape.id, {
          x: e.target.x() - rx,
          y: e.target.y() - ry,
        });
      }}
      onTransformEnd={() => {
        const node = ref.current;
        if (!node) return;
        const scaleX = node.scaleX();
        const scaleY = node.scaleY();
        node.scaleX(1);
        node.scaleY(1);
        const newRx = Math.max(3, node.radiusX() * scaleX);
        const newRy = Math.max(3, node.radiusY() * scaleY);
        updateShape(shape.id, {
          x: node.x() - newRx,
          y: node.y() - newRy,
          width: newRx * 2,
          height: newRy * 2,
        });
      }}
    />
  );
}
