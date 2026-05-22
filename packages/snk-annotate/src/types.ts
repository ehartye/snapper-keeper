export type AnnotationTool =
  | 'select'
  | 'arrow'
  | 'rectangle'
  | 'ellipse'
  | 'pen'
  | 'highlighter'
  | 'text'
  | 'blur'
  | 'step-marker'
  | 'crop';

export interface StrokeConfig {
  color: string;
  width: number;
  opacity: number;
}

export interface AnnotationShape {
  id: string;
  tool: AnnotationTool;
  points?: number[];
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  text?: string;
  stroke: StrokeConfig;
  stepNumber?: number;
  rotation?: number;
}
