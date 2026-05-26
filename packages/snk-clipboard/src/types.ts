export interface ClipboardItem {
  id: string;
  kind: 'text' | 'image';
  text_content: string | null;
  file_path: string | null;
  content_hash: string;
  source_app: string | null;
  source_window_title: string | null;
  created_at: number;
  pinned: boolean;
}

export interface ListClipboardQuery {
  limit?: number;
  filter?: string;
}

export interface CaretPosition {
  x: number;
  y: number;
}

export type SourceAppKind = 'macos_bundle_id' | 'windows_exe';

export interface SourceApp {
  identifier: string;
  display_name: string;
  kind: SourceAppKind;
}

export interface BlocklistEntry {
  identifier: string;
  display_name: string;
  kind: SourceAppKind;
}
