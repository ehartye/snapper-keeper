import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock @tauri-apps/api/core before importing the binding module.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import {
  listCaptures,
  getCapture,
  softDeleteCapture,
  hardDeleteCapture,
  purgeTrash,
  setCapturePinned,
  searchLibrary,
  listTags,
  createTag,
  updateTag,
  deleteTag,
  assignTag,
  removeTag,
  listCaptureTags,
  getSetting,
  setSetting,
} from './index';

const mockedInvoke = vi.mocked(invoke);

describe('@snk/library bindings', () => {
  beforeEach(() => {
    mockedInvoke.mockClear();
    mockedInvoke.mockResolvedValue(undefined);
  });

  describe('captures', () => {
    it('listCaptures forwards an optional query', async () => {
      mockedInvoke.mockResolvedValue([]);
      const res = await listCaptures({ limit: 50, pinned_only: true });
      expect(res).toEqual([]);
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|list_captures', {
        query: { limit: 50, pinned_only: true },
      });
    });

    it('listCaptures passes undefined query through', async () => {
      mockedInvoke.mockResolvedValue([]);
      await listCaptures();
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|list_captures', {
        query: undefined,
      });
    });

    it('getCapture sends the id', async () => {
      mockedInvoke.mockResolvedValue({ id: 'abc' });
      await getCapture('abc');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|get_capture', {
        id: 'abc',
      });
    });

    it('softDeleteCapture sends the id', async () => {
      await softDeleteCapture('abc');
      expect(mockedInvoke).toHaveBeenCalledWith(
        'plugin:snk-library|soft_delete_capture',
        { id: 'abc' },
      );
    });

    it('hardDeleteCapture sends the id', async () => {
      await hardDeleteCapture('abc');
      expect(mockedInvoke).toHaveBeenCalledWith(
        'plugin:snk-library|hard_delete_capture',
        { id: 'abc' },
      );
    });

    it('purgeTrash takes no args, returns the count', async () => {
      mockedInvoke.mockResolvedValue(7);
      const n = await purgeTrash();
      expect(n).toBe(7);
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|purge_trash');
    });

    it('setCapturePinned sends id + pinned', async () => {
      await setCapturePinned('abc', true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        'plugin:snk-library|set_capture_pinned',
        { id: 'abc', pinned: true },
      );
    });
  });

  describe('search', () => {
    it('searchLibrary forwards query and limit', async () => {
      mockedInvoke.mockResolvedValue([]);
      await searchLibrary('hello', 10);
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|search_library', {
        query: 'hello',
        limit: 10,
      });
    });

    it('searchLibrary leaves limit undefined when omitted', async () => {
      mockedInvoke.mockResolvedValue([]);
      await searchLibrary('hi');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|search_library', {
        query: 'hi',
        limit: undefined,
      });
    });
  });

  describe('tags', () => {
    it('listTags takes no args', async () => {
      mockedInvoke.mockResolvedValue([]);
      await listTags();
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|list_tags');
    });

    it('createTag sends name + color', async () => {
      mockedInvoke.mockResolvedValue({ id: 't1', name: 'work', color: '#fff' });
      await createTag('work', '#fff');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|create_tag', {
        name: 'work',
        color: '#fff',
      });
    });

    it('updateTag sends id, name, color', async () => {
      mockedInvoke.mockResolvedValue({ id: 't1' });
      await updateTag('t1', 'home', '#000');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|update_tag', {
        id: 't1',
        name: 'home',
        color: '#000',
      });
    });

    it('deleteTag sends the id', async () => {
      await deleteTag('t1');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|delete_tag', {
        id: 't1',
      });
    });

    it('assignTag camelCases the args', async () => {
      await assignTag('c1', 't1');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|assign_tag', {
        captureId: 'c1',
        tagId: 't1',
      });
    });

    it('removeTag camelCases the args', async () => {
      await removeTag('c1', 't1');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|remove_tag', {
        captureId: 'c1',
        tagId: 't1',
      });
    });

    it('listCaptureTags camelCases the arg', async () => {
      mockedInvoke.mockResolvedValue([]);
      await listCaptureTags('c1');
      expect(mockedInvoke).toHaveBeenCalledWith(
        'plugin:snk-library|list_capture_tags',
        { captureId: 'c1' },
      );
    });
  });

  describe('settings', () => {
    it('getSetting sends the key', async () => {
      mockedInvoke.mockResolvedValue('utf-8');
      const v = await getSetting('encoding');
      expect(v).toBe('utf-8');
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|get_setting', {
        key: 'encoding',
      });
    });

    it('setSetting forwards arbitrary JSON values', async () => {
      await setSetting('history_size', 500);
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'history_size',
        value: 500,
      });
    });

    it('setSetting forwards complex values', async () => {
      await setSetting('layout', { x: 1, y: 2 });
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'layout',
        value: { x: 1, y: 2 },
      });
    });
  });
});
