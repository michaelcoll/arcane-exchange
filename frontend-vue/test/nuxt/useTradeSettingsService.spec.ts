import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useTradeSettingsService } from '~/composables/useTradeSettingsService';

const { apiCallMock } = vi.hoisted(() => ({
  apiCallMock: vi.fn(),
}));

vi.mock('~/composables/useApi', () => ({
  useApi: () => ({ apiCall: apiCallMock }),
}));

describe('useTradeSettingsService', () => {
  beforeEach(() => {
    apiCallMock.mockReset();
    apiCallMock.mockResolvedValue(undefined);
  });

  it('lit et modifie la visibilité sous /collection/trade-settings', async () => {
    const { getVisibility, setVisibility } = useTradeSettingsService();

    await getVisibility();
    await setVisibility('trade');

    expect(apiCallMock).toHaveBeenNthCalledWith(1, '/collection/trade-settings/visibility');
    expect(apiCallMock).toHaveBeenNthCalledWith(2, '/collection/trade-settings/visibility', {
      method: 'PUT',
      body: { visibility: 'trade' },
    });
  });

  it('lit, ajoute et retire les binders ouverts à l’échange', async () => {
    const { getTradeBinders, addTradeBinder, removeTradeBinder } = useTradeSettingsService();

    await getTradeBinders();
    await addTradeBinder('Trade Binder');
    await removeTradeBinder('Mes rares/foils');

    expect(apiCallMock).toHaveBeenNthCalledWith(1, '/collection/trade-settings/binders');
    expect(apiCallMock).toHaveBeenNthCalledWith(2, '/collection/trade-settings/binders', {
      method: 'POST',
      body: { binder_name: 'Trade Binder' },
    });
    expect(apiCallMock).toHaveBeenNthCalledWith(
      3,
      '/collection/trade-settings/binders/Mes%20rares%2Ffoils',
      { method: 'DELETE' },
    );
  });

  it('lit et modifie les filtres de rareté', async () => {
    apiCallMock.mockResolvedValue({ rarities: [] });
    const { getRarityFilters, setRarityFilter } = useTradeSettingsService();

    await getRarityFilters();
    await setRarityFilter('M', true, 2);

    expect(apiCallMock).toHaveBeenNthCalledWith(1, '/collection/trade-settings/rarities');
    expect(apiCallMock).toHaveBeenNthCalledWith(2, '/collection/trade-settings/rarities', {
      method: 'POST',
      body: { rarity: 'M', is_open: true, kept_copies: 2 },
    });
  });
});
