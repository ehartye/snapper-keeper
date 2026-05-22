import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import { formatTimeAgo } from './formatTimeAgo';

describe('formatTimeAgo', () => {
  const NOW = new Date('2026-05-22T12:00:00Z').getTime();

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(NOW);
  });
  afterEach(() => vi.useRealTimers());

  it("returns 'just now' for the present second", () => {
    expect(formatTimeAgo(NOW)).toBe('just now');
    expect(formatTimeAgo(NOW - 1)).toBe('just now');
    expect(formatTimeAgo(NOW - 59_000)).toBe('just now');
  });

  it('returns minutes for 1-59 minutes', () => {
    expect(formatTimeAgo(NOW - 60_000)).toBe('1m ago');
    expect(formatTimeAgo(NOW - 30 * 60_000)).toBe('30m ago');
    expect(formatTimeAgo(NOW - 59 * 60_000)).toBe('59m ago');
  });

  it('returns hours for 1-23 hours', () => {
    expect(formatTimeAgo(NOW - 60 * 60_000)).toBe('1h ago');
    expect(formatTimeAgo(NOW - 12 * 60 * 60_000)).toBe('12h ago');
    expect(formatTimeAgo(NOW - 23 * 60 * 60_000)).toBe('23h ago');
  });

  it('returns days for 24h+', () => {
    expect(formatTimeAgo(NOW - 24 * 60 * 60_000)).toBe('1d ago');
    expect(formatTimeAgo(NOW - 7 * 24 * 60 * 60_000)).toBe('7d ago');
  });
});
