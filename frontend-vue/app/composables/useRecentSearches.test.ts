import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  getRecentSearches,
  saveRecentSearches,
  addRecentSearch,
} from '../composables/useRecentSearches';

const TEST_KEY = 'test-recent-key';

// Mock localStorage
const mockLocalStorage: Record<string, string> = {};
vi.stubGlobal('localStorage', {
  get length(): number {
    return Object.keys(mockLocalStorage).length;
  },
  getItem: vi.fn((key: string) => mockLocalStorage[key] ?? null),
  setItem: vi.fn((key: string, value: string) => {
    mockLocalStorage[key] = value;
  }),
  removeItem: vi.fn((key: string) => {
    delete mockLocalStorage[key];
  }),
  clear: vi.fn(() => {
    Object.keys(mockLocalStorage).forEach((k) => delete mockLocalStorage[k]);
  }),
  key: vi.fn((n: number) => Object.keys(mockLocalStorage)[n] ?? null),
} satisfies Storage);

function clearStorage(): void {
  Object.keys(mockLocalStorage).forEach((k) => delete mockLocalStorage[k]);
  vi.mocked(localStorage.getItem).mockReset();
  vi.mocked(localStorage.getItem).mockImplementation(
    (key: string) => mockLocalStorage[key] ?? null,
  );
  vi.mocked(localStorage.setItem).mockReset();
  vi.mocked(localStorage.setItem).mockImplementation((key: string, value: string) => {
    mockLocalStorage[key] = value;
  });
}

describe('useRecentSearches', () => {
  beforeEach(() => {
    clearStorage();
  });

  describe('getRecentSearches', () => {
    it('returns empty array when localStorage is empty', () => {
      expect(getRecentSearches(TEST_KEY)).toEqual([]);
    });

    it('returns parsed array from localStorage', () => {
      mockLocalStorage[TEST_KEY] = JSON.stringify(['A', 'B', 'C']);
      expect(getRecentSearches(TEST_KEY)).toEqual(['A', 'B', 'C']);
    });

    it('returns empty array for non-array JSON value', () => {
      mockLocalStorage[TEST_KEY] = JSON.stringify('not-an-array');
      expect(getRecentSearches(TEST_KEY)).toEqual([]);
    });

    it('returns empty array for corrupted JSON', () => {
      mockLocalStorage[TEST_KEY] = '{invalid json';
      expect(getRecentSearches(TEST_KEY)).toEqual([]);
    });

    it('returns empty array when localStorage is disabled', () => {
      vi.mocked(localStorage.getItem).mockImplementation(() => {
        throw new Error('localStorage not available');
      });
      expect(getRecentSearches(TEST_KEY)).toEqual([]);
    });
  });

  describe('saveRecentSearches', () => {
    it('saves array as JSON string to localStorage', () => {
      saveRecentSearches(TEST_KEY, ['A', 'B']);
      expect(localStorage.setItem).toHaveBeenCalledWith(TEST_KEY, JSON.stringify(['A', 'B']));
    });

    it('truncates to MAX_RECENTS (4) entries', () => {
      saveRecentSearches(TEST_KEY, ['A', 'B', 'C', 'D', 'E']);
      expect(localStorage.setItem).toHaveBeenCalledWith(
        TEST_KEY,
        JSON.stringify(['A', 'B', 'C', 'D']),
      );
    });

    it('silently ignores errors when localStorage is disabled', () => {
      vi.mocked(localStorage.setItem).mockImplementation(() => {
        throw new Error('localStorage not available');
      });
      expect(() => saveRecentSearches(TEST_KEY, ['A'])).not.toThrow();
    });
  });

  describe('addRecentSearch', () => {
    it('adds a new term at the beginning', () => {
      const result = addRecentSearch(TEST_KEY, ['B', 'C'], 'A');
      expect(result).toEqual(['A', 'B', 'C']);
    });

    it('moves existing term to the beginning', () => {
      const result = addRecentSearch(TEST_KEY, ['A', 'B', 'C'], 'A');
      expect(result).toEqual(['A', 'B', 'C']);
    });

    it('moves existing term to the beginning when not first', () => {
      const result = addRecentSearch(TEST_KEY, ['B', 'A', 'C'], 'A');
      expect(result).toEqual(['A', 'B', 'C']);
    });

    it('truncates to MAX_RECENTS (4) entries', () => {
      const result = addRecentSearch(TEST_KEY, ['A', 'B', 'C', 'D'], 'E');
      expect(result).toEqual(['E', 'A', 'B', 'C']);
    });

    it('ignores empty or whitespace-only terms', () => {
      const result = addRecentSearch(TEST_KEY, ['A', 'B'], '');
      expect(result).toEqual(['A', 'B']);
    });

    it('ignores whitespace-only terms', () => {
      const result = addRecentSearch(TEST_KEY, ['A', 'B'], '   ');
      expect(result).toEqual(['A', 'B']);
    });

    it('trims the term before storing', () => {
      const result = addRecentSearch(TEST_KEY, [], '  Vampiric Tutor  ');
      expect(result).toEqual(['Vampiric Tutor']);
    });

    it('saves to localStorage', () => {
      clearStorage();
      addRecentSearch(TEST_KEY, [], 'Test');
      expect(localStorage.setItem).toHaveBeenCalled();
    });
  });

  describe('key isolation', () => {
    it('keeps separate histories for different keys', () => {
      addRecentSearch('key-a', [], 'Alpha');
      addRecentSearch('key-b', [], 'Beta');

      expect(getRecentSearches('key-a')).toEqual(['Alpha']);
      expect(getRecentSearches('key-b')).toEqual(['Beta']);
    });
  });
});
