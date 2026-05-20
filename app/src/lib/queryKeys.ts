export const queryKeys = {
  captures: {
    list: (query?: { limit?: number; include_deleted?: boolean }) =>
      ['captures', 'list', query ?? {}] as const,
    one: (id: string) => ['captures', 'one', id] as const,
  },
};
