import { useEffect, useRef } from 'react';
import { Line } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

export function HighlighterShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Line | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Line
      ref={ref}
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width * 4}
      opacity={0.35}
      lineCap="round"
      lineJoin="round"
      tension={0.5}
      globalCompositeOperation="multiply"
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
      onTransformEnd={() => {
        const node = ref.current;
        if (!node) return;
        const tx = node.x();
        const ty = node.y();
        const sx = node.scaleX();
        const sy = node.scaleY();
        const pts = (shape.points ?? []).map((v, i) =>
          i % 2 === 0 ? tx + v * sx : ty + v * sy,
        );
        node.x(0);
        node.y(0);
        node.scaleX(1);
        node.scaleY(1);
        updateShape(shape.id, { points: pts });
      }}
    />
  );
}
