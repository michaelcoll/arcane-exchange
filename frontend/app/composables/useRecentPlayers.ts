import type { UserSuggestion } from '~/bindings/UserSuggestion';

const MAX_RECENTS = 4;

export const PLAYER_RECENT_SEARCHES_KEY = 'tae_recent_players';

function isUserSuggestion(value: unknown): value is UserSuggestion {
  if (!value || typeof value !== 'object') return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.username === 'string' && typeof v.note === 'number' && typeof v.card_count === 'number'
  );
}

export function getRecentPlayers(key: string): UserSuggestion[] {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter(isUserSuggestion) : [];
  } catch {
    return [];
  }
}

export function saveRecentPlayers(key: string, players: UserSuggestion[]): void {
  try {
    localStorage.setItem(key, JSON.stringify(players.slice(0, MAX_RECENTS)));
  } catch {
    // localStorage désactivé — silently ignore
  }
}

export function addRecentPlayer(
  key: string,
  recents: UserSuggestion[],
  player: UserSuggestion,
): UserSuggestion[] {
  if (!player.username.trim()) return recents;
  const filtered = recents.filter((p) => p.username !== player.username);
  const updated = [player, ...filtered].slice(0, MAX_RECENTS);
  saveRecentPlayers(key, updated);
  return updated;
}

export function useRecentPlayers(key: string) {
  const recents = ref<UserSuggestion[]>(getRecentPlayers(key));

  function addRecentPlayerWrapper(player: UserSuggestion): void {
    recents.value = addRecentPlayer(key, recents.value, player);
  }

  return {
    recents,
    addRecentPlayer: addRecentPlayerWrapper,
  };
}
