import { useEffect, useRef } from 'react';
import { Circle, Text, Group } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

// Defaults match the initial draw size; user-scaled markers store their
// resolved width/height back on the shape and we re-render from those.
const DEFAULT_DIAMETER = 32;
const BASE_FONT_SIZE = 18;

export function StepMarkerShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Group | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);
  const num = shape.stepNumber ?? 1;

  // The marker stays a circle even if the user makes the bounding box
  // non-square — radius tracks the shorter side.
  const width = shape.width ?? DEFAULT_DIAMETER;
  const height = shape.height ?? DEFAULT_DIAMETER;
  const radius = Math.min(width, height) / 2;
  const fontSize = (radius / (DEFAULT_DIAMETER / 2)) * BASE_FONT_SIZE;

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Group
      ref={ref}
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      width={width}
      height={height}
      offsetX={width / 2}
      offsetY={height / 2}
      draggable={draggable}
      onMouseDown={onSelect}
      onTap={onSelect}
      onDragEnd={(e) => {
        updateShape(shape.id, { x: e.target.x(), y: e.target.y() });
      }}
      onTransformEnd={() => {
        const node = ref.current;
        if (!node) return;
        const sx = node.scaleX();
        const sy = node.scaleY();
        node.scaleX(1);
        node.scaleY(1);
        updateShape(shape.id, {
          x: node.x(),
          y: node.y(),
          width: Math.max(8, width * sx),
          height: Math.max(8, height * sy),
        });
      }}
    >
      {/* Center the visual content inside the bounding box (offsetX/Y above
          shifts the origin so x/y points to the marker's center, matching
          the previous behavior). */}
      <Circle x={width / 2} y={height / 2} radius={radius} fill={shape.stroke.color} />
      <Text
        x={width / 2 - radius}
        y={height / 2 - radius}
        width={radius * 2}
        height={radius * 2}
        text={String(num)}
        fontSize={fontSize}
        fontStyle="bold"
        fill="#ffffff"
        fontFamily="system-ui, sans-serif"
        align="center"
        verticalAlign="middle"
      />
    </Group>
  );
}
