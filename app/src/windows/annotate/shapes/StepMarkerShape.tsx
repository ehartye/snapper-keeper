import { useEffect, useRef } from 'react';
import { Circle, Text, Group } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

export function StepMarkerShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Group | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);
  const radius = 16;
  const num = shape.stepNumber ?? 1;

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Group
      ref={ref}
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      draggable={draggable}
      onMouseDown={onSelect}
      onTap={onSelect}
      onDragEnd={(e) => {
        updateShape(shape.id, { x: e.target.x(), y: e.target.y() });
      }}
    >
      <Circle radius={radius} fill={shape.stroke.color} />
      <Text
        x={-radius}
        y={-radius}
        width={radius * 2}
        height={radius * 2}
        text={String(num)}
        fontSize={18}
        fontStyle="bold"
        fill="#ffffff"
        fontFamily="system-ui, sans-serif"
        align="center"
        verticalAlign="middle"
      />
    </Group>
  );
}
