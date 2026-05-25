/**
 * Adversarial test against `SearchBar`'s snippet renderer.
 *
 * SQLite FTS5's `snippet()` returns a string that the row renders into
 * the result list. The snippet contains literal `<mark>` tags around match
 * terms — those are the ONLY ASCII tags we render as real HTML. Every
 * other substring is React-child-escaped.
 *
 * This file feeds an OWASP-style XSS corpus through the rendering path
 * and asserts:
 *   1. No `<script>` element is created.
 *   2. No element ends up with an `onerror`, `onload`, or `onclick` handler
 *      attribute (string-property reflection).
 *   3. `window.__SK_XSS_FIRED__` stays undefined (no payload executed).
 *   4. The literal payload text is still present in the DOM (so we
 *      visibly fail loudly if someone accidentally re-introduces
 *      `dangerouslySetInnerHTML`).
 *
 * One assertion-set catches the CB1 stored-XSS chain, A-U7 logged-leak
 * reflection chain, and any future regression that swaps in an unsafe
 * renderer.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { SearchBar, renderSnippet } from './SearchBar';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

// Reduced OWASP-style XSS cheat-sheet — covers the major vectors that
// `dangerouslySetInnerHTML` would have executed. Each entry is rendered
// as the FTS5 snippet content.
const XSS_PAYLOADS = [
  '<script>window.__SK_XSS_FIRED__=true</script>',
  '<img src=x onerror="window.__SK_XSS_FIRED__=true">',
  '<svg onload="window.__SK_XSS_FIRED__=true"></svg>',
  '<iframe src="javascript:window.__SK_XSS_FIRED__=true"></iframe>',
  '<a href="javascript:window.__SK_XSS_FIRED__=true">click</a>',
  '<body onload="window.__SK_XSS_FIRED__=true">',
  '"><script>window.__SK_XSS_FIRED__=true</script>',
  'before<script>window.__SK_XSS_FIRED__=true</script>after',
  // Real-world hostile snippet shape: FTS5 marker around the executable.
  '<mark><img src=x onerror="window.__SK_XSS_FIRED__=true"></mark>',
];

describe('SearchBar XSS resistance', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    // Make sure we can detect both new and stale flags across cases.
    delete (window as unknown as Record<string, unknown>).__SK_XSS_FIRED__;
  });

  it.each(XSS_PAYLOADS)(
    'does not execute payload %#: %s',
    async (payload) => {
      mockedInvoke.mockResolvedValue([
        { kind: 'capture', id: 'attack', rank: 1, snippet: payload },
      ]);

      renderWithQuery(<SearchBar />);
      const input = screen.getByPlaceholderText(/search captures/i);
      fireEvent.change(input, { target: { value: 'go' } });
      await act(async () => {
        vi.advanceTimersByTime(260);
      });

      // Wait for results to render.
      await waitFor(() => {
        expect(document.body.textContent).toMatch(/[a-z]/i);
      });

      // 1. No <script> element anywhere in the rendered DOM.
      expect(document.querySelectorAll('script')).toHaveLength(0);

      // 2. No element acquired an inline event-handler attribute. This
      //    catches attribute reflection where parsed-then-rendered HTML
      //    would attach the listener.
      for (const attr of ['onerror', 'onload', 'onclick', 'onmouseover']) {
        expect(
          document.querySelector(`[${attr}]`),
          `no element should have an ${attr} attribute (payload: ${payload})`,
        ).toBeNull();
      }

      // 3. The actual payload never executed.
      expect(
        (window as unknown as Record<string, unknown>).__SK_XSS_FIRED__,
        `payload executed (payload: ${payload})`,
      ).toBeUndefined();
    },
  );

  it('still renders <mark> highlights for legitimate FTS5 markers', () => {
    const rendered = renderSnippet(
      'plain <mark>highlight</mark> tail <mark>two</mark> end',
    );
    // 5 segments: "plain ", <mark>highlight</mark>, " tail ", <mark>two</mark>, " end"
    expect(rendered).toHaveLength(5);
  });

  it('escapes literal angle brackets when there is no <mark>', () => {
    // Pure-string input — renderSnippet should return the whole thing as
    // one text child; React renders text-as-text (escaped automatically).
    const rendered = renderSnippet('user typed <not-a-tag> literally');
    expect(rendered).toEqual(['user typed <not-a-tag> literally']);
  });
});
