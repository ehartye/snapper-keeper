import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';

import { AnnotateToolbar } from './AnnotateToolbar';
import { useAnnotateStore, COLORS } from './store';

describe('<AnnotateToolbar />', () => {
  beforeEach(() => useAnnotateStore.getState().reset());

  it('renders all tool buttons', () => {
    render(<AnnotateToolbar />);
    for (const label of [
      'Select',
      'Arrow',
      'Rectangle',
      'Ellipse',
      'Pen',
      'Highlighter',
      'Text',
      'Blur',
      'Step',
      'Crop',
    ]) {
      expect(screen.getByTitle(label)).toBeInTheDocument();
    }
  });

  it('clicking a tool updates the store', () => {
    render(<AnnotateToolbar />);
    fireEvent.click(screen.getByTitle('Rectangle'));
    expect(useAnnotateStore.getState().tool).toBe('rectangle');
    fireEvent.click(screen.getByTitle('Pen'));
    expect(useAnnotateStore.getState().tool).toBe('pen');
  });

  it('toggles the color picker popover when the swatch is clicked', () => {
    render(<AnnotateToolbar />);
    // No popover initially
    expect(screen.queryAllByTitle(/^#/i).length).toBeLessThanOrEqual(0);
    fireEvent.click(screen.getByTitle('Color'));
    // After opening, every color swatch is rendered as a button with the
    // color value as the title.
    expect(screen.getByTitle(COLORS[0])).toBeInTheDocument();
  });

  it('selecting a color from the popover updates the store and closes', () => {
    render(<AnnotateToolbar />);
    fireEvent.click(screen.getByTitle('Color'));
    fireEvent.click(screen.getByTitle(COLORS[2]));
    expect(useAnnotateStore.getState().color).toBe(COLORS[2]);
    // Popover should be closed afterwards: COLOR swatches are gone again.
    expect(screen.queryByTitle(COLORS[0])).toBeNull();
  });

  it('clicking outside the popover closes it', () => {
    render(<AnnotateToolbar />);
    fireEvent.click(screen.getByTitle('Color'));
    expect(screen.getByTitle(COLORS[0])).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByTitle(COLORS[0])).toBeNull();
  });

  it('stroke preset buttons update the store', () => {
    render(<AnnotateToolbar />);
    fireEvent.click(screen.getByTitle('thick'));
    expect(useAnnotateStore.getState().strokePreset).toBe('thick');
    fireEvent.click(screen.getByTitle('thin'));
    expect(useAnnotateStore.getState().strokePreset).toBe('thin');
  });

  it('undo button is disabled when the stack is empty', () => {
    render(<AnnotateToolbar />);
    const undo = screen.getByTitle(/Undo/i);
    expect(undo).toBeDisabled();
  });

  it('undo button is enabled after a shape is added', () => {
    useAnnotateStore.getState().addShape({
      id: 'sh',
      tool: 'rectangle',
      x: 0,
      y: 0,
      width: 10,
      height: 10,
      stroke: { color: '#000', width: 1, opacity: 1 },
    });
    render(<AnnotateToolbar />);
    const undo = screen.getByTitle(/Undo/i);
    expect(undo).not.toBeDisabled();
  });

  it('clicking undo invokes the store undo action', () => {
    const s = useAnnotateStore.getState();
    s.addShape({
      id: 'sh',
      tool: 'rectangle',
      x: 0,
      y: 0,
      width: 10,
      height: 10,
      stroke: { color: '#000', width: 1, opacity: 1 },
    });
    expect(useAnnotateStore.getState().shapes).toHaveLength(1);
    render(<AnnotateToolbar />);
    fireEvent.click(screen.getByTitle(/Undo/i));
    expect(useAnnotateStore.getState().shapes).toHaveLength(0);
  });
});
