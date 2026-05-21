import { Circle, Text, Group } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function StepMarkerShape({ shape }: Props) {
  const radius = 16;
  const num = shape.stepNumber ?? 1;
  return (
    <Group x={shape.x ?? 0} y={shape.y ?? 0}>
      <Circle
        radius={radius}
        fill={shape.stroke.color}
      />
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
