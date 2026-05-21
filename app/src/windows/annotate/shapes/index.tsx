import type { AnnotationShape } from '@snk/annotate';

import { ArrowShape } from './ArrowShape';
import { RectangleShape } from './RectangleShape';
import { EllipseShape } from './EllipseShape';
import { PenShape } from './PenShape';
import { HighlighterShape } from './HighlighterShape';
import { TextShape } from './TextShape';
import { BlurShape } from './BlurShape';
import { StepMarkerShape } from './StepMarkerShape';

interface Props {
  shape: AnnotationShape;
}

export function ShapeRenderer({ shape }: Props) {
  switch (shape.tool) {
    case 'arrow':
      return <ArrowShape shape={shape} />;
    case 'rectangle':
      return <RectangleShape shape={shape} />;
    case 'ellipse':
      return <EllipseShape shape={shape} />;
    case 'pen':
      return <PenShape shape={shape} />;
    case 'highlighter':
      return <HighlighterShape shape={shape} />;
    case 'text':
      return <TextShape shape={shape} />;
    case 'blur':
      return <BlurShape shape={shape} />;
    case 'step-marker':
      return <StepMarkerShape shape={shape} />;
    default:
      return null;
  }
}
