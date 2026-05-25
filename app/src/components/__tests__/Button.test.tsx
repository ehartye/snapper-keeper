import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Button } from '../Button';

describe('<Button />', () => {
  it('renders children as label', () => {
    render(<Button onClick={() => {}}>Save</Button>);
    expect(screen.getByRole('button', { name: 'Save' })).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>OK</Button>);
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('applies primary styling by default', () => {
    render(<Button onClick={() => {}}>P</Button>);
    expect(screen.getByRole('button')).toHaveClass('bg-primary');
  });

  it('applies secondary styling when variant="secondary"', () => {
    render(<Button variant="secondary" onClick={() => {}}>S</Button>);
    expect(screen.getByRole('button')).toHaveClass('bg-surface-2');
  });

  it('applies danger styling when variant="danger"', () => {
    render(<Button variant="danger" onClick={() => {}}>D</Button>);
    expect(screen.getByRole('button')).toHaveClass('bg-red-600');
  });

  it('is disabled when disabled prop is true', () => {
    render(<Button disabled onClick={() => {}}>X</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('does not call onClick when disabled', () => {
    const onClick = vi.fn();
    render(<Button disabled onClick={onClick}>X</Button>);
    fireEvent.click(screen.getByRole('button'));
    expect(onClick).not.toHaveBeenCalled();
  });

  it('forwards type prop (default "button")', () => {
    const { rerender } = render(<Button onClick={() => {}}>X</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('type', 'button');
    rerender(<Button type="submit" onClick={() => {}}>X</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('type', 'submit');
  });
});
