import type { AnnotationShape } from '@snk/annotate';

import { ArrowShape } from './ArrowShape';
import { RectangleShape } from './RectangleShape';
import { EllipseShape } from './EllipseShape';
import { PenShape } from './PenShape';
import { HighlighterShape } from './HighlighterShape';
import { TextShape } from './TextShape';
import { BlurShape } from './BlurShape';
import { StepMarkerShape } from './StepMarkerShape';

export interface ShapeProps {
  shape: AnnotationShape;
  draggable?: boolean;
  onSelect?: () => void;
  registerNode?: (node: unknown | null) => void;
}

interface RendererProps extends ShapeProps {
  onStartEditText?: () => void;
}

export function ShapeRenderer({
  shape,
  draggable,
  onSelect,
  registerNode,
  onStartEditText,
}: RendererProps) {
  const common = { shape, draggable, onSelect, registerNode };
  switch (shape.tool) {
    case 'arrow':
      return <ArrowShape {...common} />;
    case 'rectangle':
      return <RectangleShape {...common} />;
    case 'ellipse':
      return <EllipseShape {...common} />;
    case 'pen':
      return <PenShape {...common} />;
    case 'highlighter':
      return <HighlighterShape {...common} />;
    case 'text':
      return <TextShape {...common} onStartEdit={onStartEditText} />;
    case 'blur':
      return <BlurShape {...common} />;
    case 'step-marker':
      return <StepMarkerShape {...common} />;
    default:
      return null;
  }
}
