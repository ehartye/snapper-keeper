import { describe, it, expect, vi } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { FirstRunWizard } from './FirstRunWizard';
import { renderWithQuery } from '../../test/renderWithQuery';

describe('<FirstRunWizard />', () => {
  it('starts on the welcome step', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    expect(screen.getByText(/welcome to/i)).toBeInTheDocument();
    expect(screen.getByText(/get started/i)).toBeInTheDocument();
  });

  it('walks through welcome → theme → hotkeys → library → done', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    expect(screen.getByRole('heading', { name: /pick a theme/i })).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/keyboard shortcuts/i)).toBeInTheDocument();
    expect(screen.getByText('Capture region')).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/library location/i)).toBeInTheDocument();
    expect(screen.getByText(/%APPDATA%/i)).toBeInTheDocument();

    fireEvent.click(screen.getByText(/^next$/i));
    expect(screen.getByText(/all set/i)).toBeInTheDocument();
  });

  it('back buttons return to the previous step', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    fireEvent.click(screen.getByText(/^next$/i)); // -> hotkeys
    fireEvent.click(screen.getByText(/^next$/i)); // -> library
    fireEvent.click(screen.getByText(/back/i));   // -> hotkeys
    expect(screen.getByText(/keyboard shortcuts/i)).toBeInTheDocument();
    fireEvent.click(screen.getByText(/back/i));   // -> theme
    expect(screen.getByRole('heading', { name: /pick a theme/i })).toBeInTheDocument();
    fireEvent.click(screen.getByText(/back/i));   // -> welcome
    expect(screen.getByText(/welcome to/i)).toBeInTheDocument();
  });

  it("calls setSetting('firstrun.completed', true) and onComplete on the final button", async () => {
    const onComplete = vi.fn();
    renderWithQuery(<FirstRunWizard onComplete={onComplete} />);
    fireEvent.click(screen.getByText(/get started/i));
    fireEvent.click(screen.getByText(/^next$/i)); // theme -> hotkeys
    fireEvent.click(screen.getByText(/^next$/i)); // hotkeys -> library
    fireEvent.click(screen.getByText(/^next$/i)); // library -> done
    fireEvent.click(screen.getByText(/start using snapper-keeper/i));

    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'firstrun.completed',
        value: true,
      });
      expect(onComplete).toHaveBeenCalled();
    });
  });

  it('theme step renders one card per family', () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    // 8 families = 8 cards. Each card has a data-testid="theme-card".
    const cards = screen.getAllByTestId('theme-card');
    expect(cards).toHaveLength(8);
  });

  it('clicking a theme card persists the selection via setSetting', async () => {
    renderWithQuery(<FirstRunWizard onComplete={vi.fn()} />);
    fireEvent.click(screen.getByText(/get started/i));
    // Click the second card (any non-default).
    const cards = screen.getAllByTestId('theme-card');
    fireEvent.click(cards[1]!);
    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        'plugin:snk-library|set_setting',
        expect.objectContaining({ key: 'theme' }),
      );
    });
  });
});
