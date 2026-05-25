import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SettingsSection } from '../SettingsSection';

describe('<SettingsSection />', () => {
  it('renders the title in an h2', () => {
    render(
      <SettingsSection title="Capture">
        <div>row</div>
      </SettingsSection>,
    );
    const heading = screen.getByRole('heading', { name: 'Capture', level: 2 });
    expect(heading).toBeInTheDocument();
  });

  it('renders children inside the card body', () => {
    render(
      <SettingsSection title="Capture">
        <div data-testid="child">child</div>
      </SettingsSection>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('applies the card classes to the body wrapper', () => {
    const { container } = render(
      <SettingsSection title="Capture">
        <div>row</div>
      </SettingsSection>,
    );
    const body = container.querySelector('.bg-surface');
    expect(body).not.toBeNull();
    expect(body).toHaveClass('rounded-xl');
    expect(body).toHaveClass('border');
    expect(body).toHaveClass('divide-y');
  });
});
