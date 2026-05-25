import { describe, it, expect, vi } from 'vitest';
import { render, screen, renderHook, act } from '@testing-library/react';
import { ModalProvider } from '../Modal/ModalProvider';
import { useModalContext } from '../Modal/ModalProvider';

describe('<ModalProvider />', () => {
  it('renders children', () => {
    render(
      <ModalProvider>
        <div data-testid="child">child</div>
      </ModalProvider>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('provides a context whose default modal state is null', () => {
    const { result } = renderHook(() => useModalContext(), {
      wrapper: ({ children }) => <ModalProvider>{children}</ModalProvider>,
    });
    expect(result.current.modal).toBeNull();
  });

  it('setModal replaces the current modal state', () => {
    const { result } = renderHook(() => useModalContext(), {
      wrapper: ({ children }) => <ModalProvider>{children}</ModalProvider>,
    });
    act(() => {
      result.current.setModal({
        kind: 'alert',
        title: 'Hi',
        body: 'Hello',
        okLabel: 'OK',
      });
    });
    expect(result.current.modal).toEqual({
      kind: 'alert',
      title: 'Hi',
      body: 'Hello',
      okLabel: 'OK',
    });
  });

  it('throws when useModalContext is used outside ModalProvider', () => {
    // Silence the React error logged by renderHook.
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => renderHook(() => useModalContext())).toThrow(/ModalProvider/);
    spy.mockRestore();
  });
});
