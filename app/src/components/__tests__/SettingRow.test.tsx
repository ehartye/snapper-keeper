import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SettingRow } from '../SettingRow';

describe('<SettingRow />', () => {
  it('renders the label', () => {
    render(
      <SettingRow label="History size">
        <input />
      </SettingRow>,
    );
    expect(screen.getByText('History size')).toBeInTheDocument();
  });

  it('renders the description when provided', () => {
    render(
      <SettingRow label="Auto-copy" description="Copy capture to clipboard">
        <input />
      </SettingRow>,
    );
    expect(screen.getByText('Copy capture to clipboard')).toBeInTheDocument();
  });

  it('does not render a description element when description is omitted', () => {
    const { container } = render(
      <SettingRow label="History size">
        <input data-testid="control" />
      </SettingRow>,
    );
    // The label div + the control div = 2 children of the inner wrapper.
    // No description div should be present.
    expect(container.querySelectorAll('div.text-\\[11px\\]')).toHaveLength(0);
  });

  it('renders the control children', () => {
    render(
      <SettingRow label="X">
        <input data-testid="control" />
      </SettingRow>,
    );
    expect(screen.getByTestId('control')).toBeInTheDocument();
  });
});
