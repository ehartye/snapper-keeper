import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import type Konva from 'konva';

import { AnnotateTopBar } from './AnnotateTopBar';
import { useAnnotateStore } from './store';

function makeStageRef(): { current: Konva.Stage | null } {
  const stage = {
    scaleX: () => 1,
    toDataURL: vi.fn(() => 'data:image/png;base64,aGVsbG8='),
    toBlob: vi.fn(() =>
      Promise.resolve(new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' })),
    ),
  } as unknown as Konva.Stage;
  return { current: stage };
}

describe('<AnnotateTopBar />', () => {
  beforeEach(() => {
    useAnnotateStore.getState().reset();
  });

  it('renders the truncated capture id', () => {
    render(<AnnotateTopBar captureId="019e4bbf-c4ed-7f30-bb91-c9e1eeb78071" stageRef={makeStageRef()} />);
    expect(screen.getByText(/019e4bbf/)).toBeInTheDocument();
  });

  it("flashes 'saved ✓' on a successful save", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.mocked(invoke).mockResolvedValue({ id: 'cap-1' });
    render(<AnnotateTopBar captureId="cap-1" stageRef={makeStageRef()} />);

    fireEvent.click(screen.getByText('save'));
    expect(await screen.findByText(/saved/i)).toBeInTheDocument();

    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    await waitFor(() => {
      expect(screen.getByText('save')).toBeInTheDocument();
    });
    vi.useRealTimers();
  });

  it("flashes 'failed' on a save error", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.mocked(invoke).mockRejectedValue(new Error('boom'));
    render(<AnnotateTopBar captureId="cap-1" stageRef={makeStageRef()} />);

    fireEvent.click(screen.getByText('save'));
    expect(await screen.findByText(/failed/i)).toBeInTheDocument();
    vi.useRealTimers();
  });

  it('writes a PNG ClipboardItem on copy and flashes copied', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const write = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(window.navigator, 'clipboard', {
      configurable: true,
      value: { write },
    });
    Object.defineProperty(window, 'ClipboardItem', {
      configurable: true,
      value: class {
        constructor(public data: Record<string, Blob>) {}
      },
    });

    render(<AnnotateTopBar captureId="cap-1" stageRef={makeStageRef()} />);
    fireEvent.click(screen.getByText('copy'));

    await waitFor(() => {
      expect(write).toHaveBeenCalled();
    });
    expect(await screen.findByText(/copied/i)).toBeInTheDocument();
    vi.useRealTimers();
  });

  it('crop-confirmed save invokes derive_capture with shifted shapes', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const mocked = vi.mocked(invoke);
    mocked.mockReset().mockResolvedValue({ id: 'cap-derived' });

    useAnnotateStore.setState({
      shapes: [
        {
          id: 'r-1',
          tool: 'rectangle',
          x: 150,
          y: 200,
          width: 30,
          height: 40,
          stroke: { color: '#000', width: 2, opacity: 1 },
        },
      ],
      cropRegion: { x: 100, y: 50, width: 200.4, height: 150.6 },
      cropConfirmed: true,
    });

    render(<AnnotateTopBar captureId="cap-parent" stageRef={makeStageRef()} />);
    fireEvent.click(screen.getByText('save'));

    await waitFor(() => {
      expect(mocked).toHaveBeenCalledWith(
        'plugin:snk-annotate|derive_capture',
        expect.objectContaining({
          parentId: 'cap-parent',
          width: 200,
          height: 151,
          stateJson: expect.any(String),
        }),
      );
    });

    const call = mocked.mock.calls.find(
      (c) => c[0] === 'plugin:snk-annotate|derive_capture',
    )!;
    const payload = call[1] as { stateJson: string };
    const parsed = JSON.parse(payload.stateJson);
    expect(parsed.crop_region).toBeNull();
    expect(parsed.crop_confirmed).toBe(false);
    expect(parsed.shapes[0].x).toBe(50);
    expect(parsed.shapes[0].y).toBe(150);

    vi.useRealTimers();
  });

  it('no-crop save still invokes save_annotation', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const mocked = vi.mocked(invoke);
    mocked.mockReset().mockResolvedValue({ id: 'cap-1' });

    render(<AnnotateTopBar captureId="cap-1" stageRef={makeStageRef()} />);
    fireEvent.click(screen.getByText('save'));

    await waitFor(() => {
      expect(mocked).toHaveBeenCalledWith(
        'plugin:snk-annotate|save_annotation',
        expect.objectContaining({ captureId: 'cap-1' }),
      );
    });

    vi.useRealTimers();
  });

  it('Done hides the window and resets the store', async () => {
    // Dirty state triggers the discard-confirm prompt; simulate the user
    // choosing to discard so handleDone proceeds to reset.
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    useAnnotateStore.getState().addShape({
      id: 'sh-1',
      tool: 'rectangle',
      x: 0,
      y: 0,
      width: 10,
      height: 10,
      stroke: { color: '#000', width: 1, opacity: 1 },
    });
    expect(useAnnotateStore.getState().shapes).toHaveLength(1);

    render(<AnnotateTopBar captureId="cap-1" stageRef={makeStageRef()} />);
    await act(async () => {
      fireEvent.click(screen.getByText('done'));
    });
    expect(useAnnotateStore.getState().shapes).toHaveLength(0);
    confirmSpy.mockRestore();
  });
});
