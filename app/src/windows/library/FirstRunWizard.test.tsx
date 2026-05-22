import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { FirstRunWizard } from './FirstRunWizard';

describe('<FirstRunWizard />', () => {
  it('starts on the welcome step', () => {
    render(<FirstRunWizard onComplete={vi.fn()} />);
    expect(screen.getByText(/welcome to/i)).toBeInTheDocument();
    expect(screen.getByText(/get started/i)).toBeInTheDocument();
  });

  it('walks through welcome → hotkeys → library → done', () => {
    render(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    expect(screen.getByText(/keyboard shortcuts/i)).toBeInTheDocument();
    expect(screen.getByText('Capture region')).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/library location/i)).toBeInTheDocument();
    expect(screen.getByText(/%APPDATA%/i)).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/all set/i)).toBeInTheDocument();
  });

  it('back buttons return to the previous step', () => {
    render(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    fireEvent.click(screen.getByText(/^next$/i)); // -> library
    fireEvent.click(screen.getByText(/back/i));
    expect(screen.getByText(/keyboard shortcuts/i)).toBeInTheDocument();
    fireEvent.click(screen.getByText(/back/i));
    expect(screen.getByText(/welcome to/i)).toBeInTheDocument();
  });

  it("calls setSetting('firstrun.completed', true) and onComplete on the final button", async () => {
    const onComplete = vi.fn();
    render(<FirstRunWizard onComplete={onComplete} />);
    fireEvent.click(screen.getByText(/get started/i));
    fireEvent.click(screen.getByText(/^next$/i));
    fireEvent.click(screen.getByText(/^next$/i));
    fireEvent.click(screen.getByText(/start using snapper-keeper/i));

    // setSetting hits invoke under the hood
    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'firstrun.completed',
        value: true,
      });
      expect(onComplete).toHaveBeenCalled();
    });
  });
});
