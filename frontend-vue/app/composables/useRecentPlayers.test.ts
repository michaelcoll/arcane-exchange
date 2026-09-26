import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  getRecentPlayers,
  saveRecentPlayers,
  addRecentPlayer,
} from '../composables/useRecentPlayers';
import type { UserSuggestion } from '~/bindings/UserSuggestion';

const TEST_KEY = 'test-recent-players-key';

const player = (username: string, card_count = 10, note = 5): UserSuggestion => ({
  username,
  card_count,
  note,
});

// Mock localStorage
const mockLocalStorage = new Map<string, string>();
vi.stubGlobal('localStorage', {
  get length(): number {
    return mockLocalStorage.size;
  },
  getItem: vi.fn((key: string) => mockLocalStorage.get(key) ?? null),
  setItem: vi.fn((key: string, value: string) => {
    mockLocalStorage.set(key, value);
  }),
  removeItem: vi.fn((key: string) => {
    mockLocalStorage.delete(key);
  }),
  clear: vi.fn(() => {
    mockLocalStorage.clear();
  }),
  key: vi.fn((n: number) => [...mockLocalStorage.keys()][n] ?? null),
} satisfies Storage);

function clearStorage(): void {
  mockLocalStorage.clear();
  vi.mocked(localStorage.getItem).mockReset();
  vi.mocked(localStorage.getItem).mockImplementation(
    (key: string) => mockLocalStorage.get(key) ?? null,
  );
  vi.mocked(localStorage.setItem).mockReset();
  vi.mocked(localStorage.setItem).mockImplementation((key: string, value: string) => {
    mockLocalStorage.set(key, value);
  });
}

describe('useRecentPlayers', () => {
  beforeEach(() => {
    clearStorage();
  });

  describe('getRecentPlayers', () => {
    it('returns empty array when localStorage is empty', () => {
      expect(getRecentPlayers(TEST_KEY)).toEqual([]);
    });

    it('returns parsed array of full player objects from localStorage', () => {
      const players = [player('alice'), player('bob')];
      mockLocalStorage.set(TEST_KEY, JSON.stringify(players));
      expect(getRecentPlayers(TEST_KEY)).toEqual(players);
    });

    it('returns empty array for non-array JSON value', () => {
      mockLocalStorage.set(TEST_KEY, JSON.stringify('not-an-array'));
      expect(getRecentPlayers(TEST_KEY)).toEqual([]);
    });

    it('returns empty array for corrupted JSON', () => {
      mockLocalStorage.set(TEST_KEY, '{invalid json');
      expect(getRecentPlayers(TEST_KEY)).toEqual([]);
    });

    it('filters out entries not matching the UserSuggestion shape', () => {
      mockLocalStorage.set(
        TEST_KEY,
        JSON.stringify([player('alice'), { username: 'bob' }, 'oops']),
      );
      expect(getRecentPlayers(TEST_KEY)).toEqual([player('alice')]);
    });

    it('returns empty array when localStorage is disabled', () => {
      vi.mocked(localStorage.getItem).mockImplementation(() => {
        throw new Error('localStorage not available');
      });
      expect(getRecentPlayers(TEST_KEY)).toEqual([]);
    });
  });

  describe('saveRecentPlayers', () => {
    it('saves array as JSON string to localStorage', () => {
      saveRecentPlayers(TEST_KEY, [player('alice')]);
      expect(localStorage.setItem).toHaveBeenCalledWith(
        TEST_KEY,
        JSON.stringify([player('alice')]),
      );
    });

    it('truncates to MAX_RECENTS (4) entries', () => {
      const players = ['a', 'b', 'c', 'd', 'e'].map((u) => player(u));
      saveRecentPlayers(TEST_KEY, players);
      expect(localStorage.setItem).toHaveBeenCalledWith(
        TEST_KEY,
        JSON.stringify(players.slice(0, 4)),
      );
    });

    it('silently ignores errors when localStorage is disabled', () => {
      vi.mocked(localStorage.setItem).mockImplementation(() => {
        throw new Error('localStorage not available');
      });
      expect(() => saveRecentPlayers(TEST_KEY, [player('alice')])).not.toThrow();
    });
  });

  describe('addRecentPlayer', () => {
    it('adds a new player at the beginning', () => {
      const result = addRecentPlayer(TEST_KEY, [player('b'), player('c')], player('a'));
      expect(result.map((p) => p.username)).toEqual(['a', 'b', 'c']);
    });

    it('moves an existing player to the beginning and refreshes its data', () => {
      const stale = player('a', 1, 1);
      const fresh = player('a', 42, 5);
      const result = addRecentPlayer(TEST_KEY, [stale, player('b')], fresh);
      expect(result).toEqual([fresh, player('b')]);
    });

    it('truncates to MAX_RECENTS (4) entries', () => {
      const result = addRecentPlayer(
        TEST_KEY,
        [player('a'), player('b'), player('c'), player('d')],
        player('e'),
      );
      expect(result.map((p) => p.username)).toEqual(['e', 'a', 'b', 'c']);
    });

    it('ignores players with an empty username', () => {
      const recents = [player('a')];
      const result = addRecentPlayer(TEST_KEY, recents, player('   '));
      expect(result).toEqual(recents);
    });

    it('saves to localStorage', () => {
      clearStorage();
      addRecentPlayer(TEST_KEY, [], player('a'));
      expect(localStorage.setItem).toHaveBeenCalled();
    });
  });
});
