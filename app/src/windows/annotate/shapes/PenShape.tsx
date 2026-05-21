import { Line } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function PenShape({ shape }: Props) {
  return (
    <Line
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
      lineCap="round"
      lineJoin="round"
      tension={0.5}
    />
  );
}
