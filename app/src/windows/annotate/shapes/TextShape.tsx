import { Text } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function TextShape({ shape }: Props) {
  return (
    <Text
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      text={shape.text ?? ''}
      fontSize={shape.stroke.width * 6}
      fill={shape.stroke.color}
      fontFamily="system-ui, sans-serif"
    />
  );
}
