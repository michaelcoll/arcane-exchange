import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import CardDetailModal from '~/components/Card/DetailModal.vue';
import type { CollectionCard } from '~/bindings/CollectionCard';
import type { RarityCode } from '~/bindings/RarityCode';

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
  the_gatherer_id: null,
  collection_entry: null,
  owner_count: null,
  reserved: false,
  price_guide: null,
};

const RARITY_COLOR_CLASS: Record<RarityCode, string> = {
  C: 'text-[var(--rarity-common)]',
  U: 'text-[var(--rarity-uncommon)]',
  R: 'text-[var(--rarity-rare)]',
  M: 'text-[var(--rarity-mythic)]',
  S: 'text-[var(--rarity-special)]',
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
});
