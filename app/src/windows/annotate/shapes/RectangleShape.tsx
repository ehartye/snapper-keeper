import { Rect } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function RectangleShape({ shape }: Props) {
  return (
    <Rect
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      width={shape.width ?? 0}
      height={shape.height ?? 0}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
    />
  );
}
