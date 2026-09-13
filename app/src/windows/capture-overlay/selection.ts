export interface Selection {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

/** Convert the rendered image's client coordinates to its actual PNG pixels. */
export function selectionPixels(
  selection: Selection,
  bounds: { left: number; top: number; width: number; height: number },
  width: number,
  height: number,
) {
  if (
    ![
      ...Object.values(selection),
      bounds.left,
      bounds.top,
      bounds.width,
      bounds.height,
      width,
      height,
    ].every(Number.isFinite) ||
    bounds.width <= 0 ||
    bounds.height <= 0 ||
    width <= 0 ||
    height <= 0 ||
    selection.startX === selection.endX ||
    selection.startY === selection.endY
  )
    return null;
  const px = (x: number) =>
    Math.max(0, Math.min(width, ((x - bounds.left) * width) / bounds.width));
  const py = (y: number) =>
    Math.max(0, Math.min(height, ((y - bounds.top) * height) / bounds.height));
  const x = Math.floor(px(Math.min(selection.startX, selection.endX)));
  const y = Math.floor(py(Math.min(selection.startY, selection.endY)));
  const right = Math.ceil(px(Math.max(selection.startX, selection.endX)));
  const bottom = Math.ceil(py(Math.max(selection.startY, selection.endY)));
  if (right <= x || bottom <= y) return null;
  return { x, y, w: right - x, h: bottom - y };
}
