import type { ListCapturesQuery } from '@snk/library';

export const queryKeys = {
  captures: {
    all: () => ['captures'] as const,
    list: (query?: ListCapturesQuery) =>
      ['captures', 'list', query ?? {}] as const,
    one: (id: string) => ['captures', 'one', id] as const,
  },
  tags: {
    all: () => ['tags'] as const,
    list: () => ['tags', 'list'] as const,
    forCapture: (captureId: string) => ['tags', 'capture', captureId] as const,
  },
  settings: {
    one: (key: string) => ['settings', key] as const,
  },
  clipboard: {
    list: () => ['clipboard', 'list'] as const,
  },
};
