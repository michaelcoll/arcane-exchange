import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import CardDetailModal from '~/components/Card/DetailModal.vue';
import type { CollectionCard } from '~/bindings/CollectionCard';
import type { RarityCode } from '~/bindings/RarityCode';
import formatPrice from '~/utils/format-price';

const {
  getCardPriceHistoryMock,
  getCardOffersMock,
  getSetMock,
  createTradeMock,
  addCardMock,
  showErrorMock,
} = vi.hoisted(() => ({
  getCardPriceHistoryMock: vi.fn(),
  getCardOffersMock: vi.fn(),
  getSetMock: vi.fn(),
  createTradeMock: vi.fn(),
  addCardMock: vi.fn(),
  showErrorMock: vi.fn(),
}));

vi.mock('~/composables/useCardsService', () => ({
  useCardsService: () => ({
    getCardPriceHistory: getCardPriceHistoryMock,
    getCardOffers: getCardOffersMock,
  }),
}));

vi.mock('~/composables/useSetsService', () => ({
  useSetsService: () => ({
    getSet: getSetMock,
  }),
}));

vi.mock('~/composables/useTradeService', () => ({
  useTradeService: () => ({
    createTrade: createTradeMock,
    addCard: addCardMock,
  }),
}));

vi.mock('~/composables/useToast', () => ({
  useToast: () => ({
    showError: showErrorMock,
  }),
}));

const stubs = {
  MtgCard: true,
  EnvelopeGraph: true,
  PlayerAvatar: true,
  Icon: true,
};

const baseCard: CollectionCard = {
  set_code: 'neo',
  collector_number: '123',
  language_code: 'en',
  foil: false,
  name: 'Test Card',
  rarity_code: 'R',
  scryfall_id: 'scryfall-1',
  image_url: null,
  image_back_url: null,
  collection_entry: null,
  owner_count: null,
  reserved: false,
  price_guide: null,
};

const RARITY_COLOR_CLASS: Record<RarityCode, string> = {
  C: 'text-rarity-common',
  U: 'text-rarity-uncommon',
  R: 'text-rarity-rare',
  M: 'text-rarity-mythic',
  S: 'text-rarity-special',
};

const NEUTRAL_COLOR_CLASS = 'text-[var(--ink-2)]';

describe('CardDetailModal', () => {
  let wrapper: VueWrapper | undefined;

  beforeEach(() => {
    getCardPriceHistoryMock.mockReset().mockResolvedValue([]);
    getCardOffersMock.mockReset().mockResolvedValue({ items: [], total: 0 });
    getSetMock.mockReset().mockResolvedValue({ code: 'NEO', name: 'Kamigawa: Neon Dynasty' });
    createTradeMock.mockReset();
    addCardMock.mockReset();
    showErrorMock.mockReset();
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    vi.useRealTimers();
  });

  it.each([false, true])(
    'asks for the price history of the card with its finish (foil: %s) over the last 30 days',
    async (foil) => {
      vi.useFakeTimers({ toFake: ['Date'] });
      vi.setSystemTime(new Date(2026, 2, 15));
      wrapper = mount(CardDetailModal, {
        props: { card: { ...baseCard, foil } },
        global: { stubs },
      });
      await flushPromises();
      expect(getCardPriceHistoryMock).toHaveBeenCalledTimes(1);
      expect(getCardPriceHistoryMock).toHaveBeenCalledWith('scryfall-1', {
        foil,
        start_date: '2026-02-14',
        end_date: '2026-03-15',
      });
    },
  );

  it('shows a rise with an up arrow and a signed, one-decimal percentage', async () => {
    getCardPriceHistoryMock.mockResolvedValue([
      { date: '2026-03-01', low: 90000, avg: 95000, trend: 100000 },
      { date: '2026-03-15', low: 90000, avg: 95000, trend: 101200 },
    ]);
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    const variation = wrapper.find('[data-testid="card-variation"]');
    expect(variation.text()).toBe('▴ +1,2 %');
    expect(variation.classes()).toContain('text-primary');
  });

  it('shows a drop with a down arrow, a true minus sign and the red tone', async () => {
    getCardPriceHistoryMock.mockResolvedValue([
      { date: '2026-03-01', low: 700, avg: 900, trend: 1000 },
      { date: '2026-03-15', low: 700, avg: 750, trend: 800 },
    ]);
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    const variation = wrapper.find('[data-testid="card-variation"]');
    expect(variation.text()).toBe('▾ −20,0 %');
    expect(variation.classes()).toContain('text-red-500');
  });

  it('keeps the trend price of the price guide above the graph, not the last trend of the history', async () => {
    getCardPriceHistoryMock.mockResolvedValue([
      { date: '2026-03-01', low: 700, avg: 900, trend: 1000 },
      { date: '2026-03-15', low: 700, avg: 750, trend: 800 },
    ]);
    wrapper = mount(CardDetailModal, {
      props: {
        card: { ...baseCard, price_guide: { low: null, avg: null, trend: 4321 } },
      },
      global: { stubs },
    });
    await flushPromises();
    expect(wrapper.find('[data-testid="card-trend-price"]').text()).toBe(formatPrice(4321));
  });

  it('says there is not enough history when the card has fewer than two entries', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    expect(wrapper.text()).toContain("Pas encore assez d'historique");
    expect(wrapper.findComponent({ name: 'EnvelopeGraph' }).exists()).toBe(false);
  });

  it('draws the detailed graph once the card has two entries', async () => {
    getCardPriceHistoryMock.mockResolvedValue([
      { date: '2026-03-01', low: 700, avg: 900, trend: 1000 },
      { date: '2026-03-15', low: 700, avg: 750, trend: 800 },
    ]);
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    const graph = wrapper.findComponent({ name: 'EnvelopeGraph' });
    expect(graph.exists()).toBe(true);
    expect(graph.props('detail')).toBe(true);
    expect(wrapper.text()).not.toContain("Pas encore assez d'historique");
  });

  it('emits "close" when clicking the backdrop', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await wrapper.trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('emits "close" when clicking the close button', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await wrapper.find('button').trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('does not emit "close" when clicking inside the panel', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await wrapper.find('h3').trigger('click');
    expect(wrapper.emitted('close')).toBeUndefined();
  });

  it('emits "close" on Escape key while mounted', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('stops listening for Escape after unmount', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    wrapper.unmount();
    expect(() =>
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })),
    ).not.toThrow();
  });

  it('displays the full set name once the /sets endpoint resolves', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    expect(wrapper.text()).toContain('Kamigawa: Neon Dynasty');
    expect(wrapper.text()).not.toContain('NEO');
  });

  it('falls back to the uppercased set code when the set is unknown to the backend', async () => {
    getSetMock.mockRejectedValue(new Error('404'));
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    expect(wrapper.text()).toContain('NEO');
  });

  it('colors the set icon neutral when the set is unknown to the backend, regardless of rarity', async () => {
    getSetMock.mockRejectedValue(new Error('404'));
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    expect(wrapper.find('i.ss').classes()).toContain(NEUTRAL_COLOR_CLASS);
  });

  it.each(Object.entries(RARITY_COLOR_CLASS) as [RarityCode, string][])(
    'colors the set icon with the token for rarity %s',
    async (rarityCode, colorClass) => {
      wrapper = mount(CardDetailModal, {
        props: { card: { ...baseCard, rarity_code: rarityCode } },
        global: { stubs },
      });
      await flushPromises();
      expect(wrapper.find('i.ss').classes()).toContain(colorClass);
    },
  );

  it('colors the set icon neutral when the rarity code is unknown', async () => {
    wrapper = mount(CardDetailModal, {
      props: { card: { ...baseCard, rarity_code: 'X' as RarityCode } },
      global: { stubs },
    });
    await flushPromises();
    expect(wrapper.find('i.ss').classes()).toContain(NEUTRAL_COLOR_CLASS);
  });

  it('says the prices and the offers failed to load for the card on screen', async () => {
    getCardPriceHistoryMock.mockRejectedValue(new Error('boom'));
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await flushPromises();
    expect(showErrorMock).toHaveBeenCalledTimes(1);
    expect(showErrorMock.mock.calls[0]?.[0]).toBe('Impossible de charger les prix et les offres');
  });

  it('stays silent when the load that fails is for a card the player has already left', async () => {
    let failFirstLoad: (reason: unknown) => void = () => {};
    getCardPriceHistoryMock.mockReturnValueOnce(
      new Promise((_, reject) => {
        failFirstLoad = reject;
      }),
    );
    wrapper = mount(CardDetailModal, {
      props: { card: baseCard },
      global: { stubs },
    });
    await wrapper.setProps({
      card: { ...baseCard, scryfall_id: 'scryfall-2', name: 'Other Card' },
    });
    await flushPromises();

    failFirstLoad(new Error('boom'));
    await flushPromises();

    expect(showErrorMock).not.toHaveBeenCalled();
  });
});
