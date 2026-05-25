import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Toggle } from '../Toggle';

describe('<Toggle />', () => {
  it('renders as a switch', () => {
    render(<Toggle value={false} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toBeInTheDocument();
  });

  it('reports the current state via aria-checked', () => {
    const { rerender } = render(<Toggle value={false} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
    rerender(<Toggle value={true} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'true');
  });

  it('calls onChange with the inverted value when clicked', () => {
    const onChange = vi.fn();
    const { rerender } = render(<Toggle value={false} onChange={onChange} />);
    fireEvent.click(screen.getByRole('switch'));
    expect(onChange).toHaveBeenCalledWith(true);

    onChange.mockClear();
    rerender(<Toggle value={true} onChange={onChange} />);
    fireEvent.click(screen.getByRole('switch'));
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it('applies the "on" style when value is true', () => {
    render(<Toggle value={true} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveClass('bg-primary');
  });

  it('applies the "off" style when value is false', () => {
    render(<Toggle value={false} onChange={() => {}} />);
    expect(screen.getByRole('switch')).toHaveClass('bg-surface-2');
  });
});
