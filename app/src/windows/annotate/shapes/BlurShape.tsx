import { Rect } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function BlurShape({ shape }: Props) {
  return (
    <Rect
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      width={shape.width ?? 0}
      height={shape.height ?? 0}
      fill="rgba(128, 128, 128, 0.6)"
      stroke="#888"
      strokeWidth={1}
      dash={[4, 4]}
    />
  );
}
