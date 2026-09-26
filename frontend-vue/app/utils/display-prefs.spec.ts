import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  CARD_SIZE_PREF,
  COLLECTION_GRAPH_RANGE_PREF,
  COLLECTION_VIEW_PREF,
  loadPreference,
  savePreference,
} from './display-prefs';

const storage = new Map<string, string>();
const localStorageMock = {
  getItem: vi.fn((key: string) => storage.get(key) ?? null),
  setItem: vi.fn((key: string, value: string) => {
    storage.set(key, value);
  }),
};
vi.stubGlobal('localStorage', localStorageMock);

const breakStorage = () => {
  localStorageMock.getItem.mockImplementationOnce(() => {
    throw new Error('SecurityError: storage disabled');
  });
  localStorageMock.setItem.mockImplementationOnce(() => {
    throw new Error('QuotaExceededError');
  });
};

describe('display preferences', () => {
  beforeEach(() => {
    storage.clear();
    vi.clearAllMocks();
  });

  it('falls back to the default when nothing is stored', () => {
    expect(loadPreference(CARD_SIZE_PREF)).toBe('md');
    expect(loadPreference(COLLECTION_VIEW_PREF)).toBe('grid');
    expect(loadPreference(COLLECTION_GRAPH_RANGE_PREF)).toBe('30 j');
  });

  it('reads back a saved value', () => {
    savePreference(CARD_SIZE_PREF, 'lg');
    expect(loadPreference(CARD_SIZE_PREF)).toBe('lg');
  });

  it('keeps each preference under its own key', () => {
    savePreference(COLLECTION_VIEW_PREF, 'list');
    savePreference(COLLECTION_GRAPH_RANGE_PREF, '1 an');
    expect(loadPreference(COLLECTION_VIEW_PREF)).toBe('list');
    expect(loadPreference(COLLECTION_GRAPH_RANGE_PREF)).toBe('1 an');
    expect(loadPreference(CARD_SIZE_PREF)).toBe('md');
  });

  it('ignores a stored value that is no longer offered', () => {
    storage.set(CARD_SIZE_PREF.key, 'xl');
    expect(loadPreference(CARD_SIZE_PREF)).toBe('md');
  });

  it('falls back to the default when the storage is unavailable', () => {
    breakStorage();
    expect(loadPreference(CARD_SIZE_PREF)).toBe('md');
    expect(() => savePreference(CARD_SIZE_PREF, 'sm')).not.toThrow();
  });

  it('works without any storage at all', () => {
    vi.stubGlobal('localStorage', undefined);
    try {
      expect(loadPreference(CARD_SIZE_PREF)).toBe('md');
      expect(() => savePreference(CARD_SIZE_PREF, 'sm')).not.toThrow();
    } finally {
      vi.stubGlobal('localStorage', localStorageMock);
    }
  });
});
