import { Line } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function HighlighterShape({ shape }: Props) {
  return (
    <Line
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width * 4}
      opacity={0.35}
      lineCap="round"
      lineJoin="round"
      tension={0.5}
      globalCompositeOperation="multiply"
    />
  );
}
