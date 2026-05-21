import { Rect, Group } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function BlurShape({ shape }: Props) {
  const x = shape.x ?? 0;
  const y = shape.y ?? 0;
  const w = shape.width ?? 0;
  const h = shape.height ?? 0;
  const blockSize = 12;

  const cols = Math.ceil(w / blockSize);
  const rows = Math.ceil(h / blockSize);
  const blocks: { bx: number; by: number; shade: string }[] = [];

  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const shade = (r + c) % 2 === 0 ? 'rgba(100,100,100,0.85)' : 'rgba(140,140,140,0.85)';
      blocks.push({ bx: x + c * blockSize, by: y + r * blockSize, shade });
    }
  }

  return (
    <Group>
      {blocks.map((b, i) => (
        <Rect
          key={i}
          x={b.bx}
          y={b.by}
          width={Math.min(blockSize, x + w - b.bx)}
          height={Math.min(blockSize, y + h - b.by)}
          fill={b.shade}
        />
      ))}
    </Group>
  );
}
