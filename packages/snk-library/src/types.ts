export interface Capture {
  id: string;
  file_path: string;
  annotated_path: string | null;
  annotation_state: string | null;
  width: number;
  height: number;
  source_app: string | null;
  source_window_title: string | null;
  monitor: string | null;
  created_at: number;
  deleted_at: number | null;
  pinned: boolean;
}

export interface ListCapturesQuery {
  limit?: number;
  include_deleted?: boolean;
  since?: number;
  pinned_only?: boolean;
  tag_id?: string;
  deleted_only?: boolean;
  /** Pagination cursor — return rows older than this created_at ms timestamp. */
  before?: number;
}

export type SearchResult =
  | { kind: 'capture'; id: string; rank: number; snippet: string }
  | { kind: 'clipboard'; id: string; rank: number; snippet: string };

export interface Tag {
  id: string;
  name: string;
  color: string;
  created_at: number;
}
