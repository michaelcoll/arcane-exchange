import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useSetsService } from '~/composables/useSetsService';

const { apiCallMock } = vi.hoisted(() => ({ apiCallMock: vi.fn() }));

vi.mock('~/composables/useApi', () => ({
  useApi: () => ({ apiCall: apiCallMock }),
}));

describe('useSetsService', () => {
  beforeEach(() => {
    apiCallMock.mockReset();
  });

  it('lists the sets the player owns a card of, through GET /sets?owned=true', async () => {
    const sets = [{ code: 'LEA', name: 'Limited Edition Alpha' }];
    apiCallMock.mockResolvedValue(sets);

    const result = await useSetsService().getOwnedSets();

    expect(apiCallMock).toHaveBeenCalledWith('/sets', { query: { owned: true } });
    expect(result).toEqual(sets);
  });

  it('keeps the API order: most recent set first, sets without a date last', async () => {
    apiCallMock.mockResolvedValue([
      { code: 'FIN', name: 'Final Fantasy' },
      { code: 'MH3', name: 'Modern Horizons 3' },
      { code: 'LEA', name: 'Limited Edition Alpha' },
    ]);

    const result = await useSetsService().getOwnedSets();

    expect(result.map((set) => set.code)).toEqual(['FIN', 'MH3', 'LEA']);
  });
});
