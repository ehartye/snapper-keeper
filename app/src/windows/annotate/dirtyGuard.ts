import { useAnnotateStore } from './store';

const DISCARD_PROMPT = 'You have unsaved annotation changes. Discard them?';

// Returns true when the caller should proceed (no dirty state, or the
// user explicitly chose to discard). Returns false when the caller must
// cancel the exit (dirty state + user clicked Cancel). Used by every
// path that leaves the annotator: the done button, the window's X
// intercept, and the Esc keyboard handler.
export function confirmDiscardIfDirty(): boolean {
  const { isDirty } = useAnnotateStore.getState();
  if (!isDirty) return true;
  return window.confirm(DISCARD_PROMPT);
}
