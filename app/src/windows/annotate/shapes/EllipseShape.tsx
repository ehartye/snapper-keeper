import { Ellipse } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function EllipseShape({ shape }: Props) {
  const rx = Math.abs((shape.width ?? 0) / 2);
  const ry = Math.abs((shape.height ?? 0) / 2);
  return (
    <Ellipse
      x={(shape.x ?? 0) + rx}
      y={(shape.y ?? 0) + ry}
      radiusX={rx}
      radiusY={ry}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
    />
  );
}
