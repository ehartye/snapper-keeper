import { useEffect, useRef } from 'react';
import { Text } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

interface Props extends ShapeProps {
  onStartEdit?: () => void;
}

export function TextShape({ shape, draggable, onSelect, registerNode, onStartEdit }: Props) {
  const ref = useRef<Konva.Text | null>(null);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <Text
      ref={ref}
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      text={shape.text ?? ''}
      fontSize={shape.stroke.width * 6}
      fill={shape.stroke.color}
      fontFamily="system-ui, sans-serif"
      draggable={draggable}
      onMouseDown={onSelect}
      onTap={onSelect}
      onDblClick={onStartEdit}
      onDblTap={onStartEdit}
      onDragEnd={(e) => {
        updateShape(shape.id, { x: e.target.x(), y: e.target.y() });
      }}
      onTransformEnd={() => {
        const node = ref.current;
        if (!node) return;
        const scaleX = node.scaleX();
        node.scaleX(1);
        node.scaleY(1);
        updateShape(shape.id, {
          x: node.x(),
          y: node.y(),
          stroke: { ...shape.stroke, width: Math.max(1, shape.stroke.width * scaleX) },
        });
      }}
    />
  );
}
