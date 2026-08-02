import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  getRecentSearches,
  saveRecentSearches,
  addRecentSearch,
} from '../composables/useRecentSearches';

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
  vi.mocked(localStorage.getItem).mockClear();
  vi.mocked(localStorage.setItem).mockClear();
}

describe('useRecentSearches', () => {
  beforeEach(() => {
    clearStorage();
  });

  describe('getRecentSearches', () => {
    it('returns empty array when localStorage is empty', () => {
      expect(getRecentSearches()).toEqual([]);
    });

    it('returns parsed array from localStorage', () => {
      mockLocalStorage['tae_recent_searches'] = JSON.stringify(['A', 'B', 'C']);
      expect(getRecentSearches()).toEqual(['A', 'B', 'C']);
    });

    it('returns empty array for non-array JSON value', () => {
      mockLocalStorage['tae_recent_searches'] = JSON.stringify('not-an-array');
      expect(getRecentSearches()).toEqual([]);
    });

    it('returns empty array for corrupted JSON', () => {
      mockLocalStorage['tae_recent_searches'] = '{invalid json';
      expect(getRecentSearches()).toEqual([]);
    });

    it('returns empty array when localStorage is disabled', () => {
      vi.mocked(localStorage.getItem).mockImplementation(() => {
        throw new Error('localStorage not available');
      });
      expect(getRecentSearches()).toEqual([]);
    });
  });

  describe('saveRecentSearches', () => {
    it('saves array as JSON string to localStorage', () => {
      saveRecentSearches(['A', 'B']);
      expect(localStorage.setItem).toHaveBeenCalledWith(
        'tae_recent_searches',
        JSON.stringify(['A', 'B']),
      );
    });

    it('truncates to MAX_RECENTS (4) entries', () => {
      saveRecentSearches(['A', 'B', 'C', 'D', 'E']);
      expect(localStorage.setItem).toHaveBeenCalledWith(
        'tae_recent_searches',
        JSON.stringify(['A', 'B', 'C', 'D']),
      );
    });

    it('silently ignores errors when localStorage is disabled', () => {
      vi.mocked(localStorage.setItem).mockImplementation(() => {
        throw new Error('localStorage not available');
      });
      expect(() => saveRecentSearches(['A'])).not.toThrow();
    });
  });

  describe('addRecentSearch', () => {
    it('adds a new term at the beginning', () => {
      const result = addRecentSearch(['B', 'C'], 'A');
      expect(result).toEqual(['A', 'B', 'C']);
    });

    it('moves existing term to the beginning', () => {
      const result = addRecentSearch(['A', 'B', 'C'], 'A');
      expect(result).toEqual(['A', 'B', 'C']);
    });

    it('moves existing term to the beginning when not first', () => {
      const result = addRecentSearch(['B', 'A', 'C'], 'A');
      expect(result).toEqual(['A', 'B', 'C']);
    });

    it('truncates to MAX_RECENTS (4) entries', () => {
      const result = addRecentSearch(['A', 'B', 'C', 'D'], 'E');
      expect(result).toEqual(['E', 'A', 'B', 'C']);
    });

    it('ignores empty or whitespace-only terms', () => {
      const result = addRecentSearch(['A', 'B'], '');
      expect(result).toEqual(['A', 'B']);
    });

    it('ignores whitespace-only terms', () => {
      const result = addRecentSearch(['A', 'B'], '   ');
      expect(result).toEqual(['A', 'B']);
    });

    it('trims the term before storing', () => {
      const result = addRecentSearch([], '  Vampiric Tutor  ');
      expect(result).toEqual(['Vampiric Tutor']);
    });

    it('saves to localStorage', () => {
      clearStorage();
      addRecentSearch([], 'Test');
      expect(localStorage.setItem).toHaveBeenCalled();
    });
  });
});
