import { useEffect, useRef } from 'react';
import { Arrow } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

export function ArrowShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Arrow | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Arrow
      ref={ref}
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
      fill={shape.stroke.color}
      pointerLength={10}
      pointerWidth={10}
      lineCap="round"
      lineJoin="round"
      draggable={draggable}
      onMouseDown={onSelect}
      onTap={onSelect}
      onDragEnd={(e) => {
        const dx = e.target.x();
        const dy = e.target.y();
        const pts = (shape.points ?? []).map((v, i) => (i % 2 === 0 ? v + dx : v + dy));
        e.target.x(0);
        e.target.y(0);
        updateShape(shape.id, { points: pts });
      }}
    />
  );
}
