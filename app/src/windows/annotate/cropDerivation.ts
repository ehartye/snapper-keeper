import type { AnnotationShape } from '@snk/annotate';

// Shift every shape's coordinates by (-cropX, -cropY) so they're
// positioned relative to a cropped image's origin instead of the
// original's. Path-tool shapes (arrow/pen/highlighter) use the
// interleaved `points` array; everything else uses x/y.
export function shiftShapesForCrop(
  shapes: AnnotationShape[],
  crop: { x: number; y: number },
): AnnotationShape[] {
  const dx = -crop.x;
  const dy = -crop.y;
  return shapes.map((s) => {
    if (s.points) {
      const shifted = s.points.map((v, i) => v + (i % 2 === 0 ? dx : dy));
      return { ...s, points: shifted };
    }
    return { ...s, x: (s.x ?? 0) + dx, y: (s.y ?? 0) + dy };
  });
}
