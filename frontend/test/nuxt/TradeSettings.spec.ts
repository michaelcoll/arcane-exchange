import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mockNuxtImport, mountSuspended } from '@nuxt/test-utils/runtime';
import { flushPromises } from '@vue/test-utils';
import TradeSettings from '~/components/Profile/TradeSettings.vue';

const { collectionService, tradeSettingsService } = vi.hoisted(() => ({
  collectionService: {
    getBinders: vi.fn(),
    getCollectionStats: vi.fn(),
  },
  tradeSettingsService: {
    getTradeBinders: vi.fn(),
    addTradeBinder: vi.fn(),
    removeTradeBinder: vi.fn(),
    getRarityFilters: vi.fn(),
    setRarityFilter: vi.fn(),
  },
}));

mockNuxtImport('useCollectionService', () => () => collectionService);
mockNuxtImport('useTradeSettingsService', () => () => tradeSettingsService);

const binder = (name: string, copies: number) => ({
  id: `id-${name}`,
  name,
  copies,
  is_open: false,
});

const mountSettings = async () => {
  const wrapper = await mountSuspended(TradeSettings);
  await flushPromises();
  return wrapper;
};

describe('Profile/TradeSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    collectionService.getBinders.mockResolvedValue({
      binders: [binder('Bulk', 12), binder('Vide', 0)],
    });
    tradeSettingsService.getTradeBinders.mockResolvedValue({ binders: [] });
    tradeSettingsService.getRarityFilters.mockResolvedValue({ rarities: [] });
    tradeSettingsService.addTradeBinder.mockResolvedValue(undefined);
  });

  it("lists the binders of the player, an empty one included, rather than the ones of the last import's entries", async () => {
    const wrapper = await mountSettings();

    const chips = wrapper.findAll('button').filter((b) => /Bulk|Vide/.test(b.text()));
    expect(chips.map((b) => b.text())).toEqual(['Bulk 12', 'Vide 0']);
    expect(collectionService.getCollectionStats).not.toHaveBeenCalled();
  });

  it('opens an empty binder to trade', async () => {
    const wrapper = await mountSettings();

    const empty = wrapper.findAll('button').find((b) => b.text().startsWith('Vide'));
    await empty!.trigger('click');
    await flushPromises();

    expect(tradeSettingsService.addTradeBinder).toHaveBeenCalledWith('Vide');
  });
});
