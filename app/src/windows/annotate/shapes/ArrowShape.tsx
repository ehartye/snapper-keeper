import { Arrow } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function ArrowShape({ shape }: Props) {
  return (
    <Arrow
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
      fill={shape.stroke.color}
      pointerLength={10}
      pointerWidth={10}
      lineCap="round"
      lineJoin="round"
    />
  );
}
