import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import CardDetailModal from '~/components/Card/DetailModal.vue';
import type { CollectionCard } from '~/bindings/CollectionCard';

const { getCardPriceHistoryMock, getCardOffersMock, createTradeMock, addCardMock, showErrorMock } =
  vi.hoisted(() => ({
    getCardPriceHistoryMock: vi.fn(),
    getCardOffersMock: vi.fn(),
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

describe('CardDetailModal', () => {
  let wrapper: VueWrapper | undefined;

  beforeEach(() => {
    getCardPriceHistoryMock.mockReset().mockResolvedValue([]);
    getCardOffersMock.mockReset().mockResolvedValue({ items: [], total: 0 });
    createTradeMock.mockReset();
    addCardMock.mockReset();
    showErrorMock.mockReset();
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
  });

  it('emits "close" when clicking the backdrop', async () => {
    wrapper = mount(CardDetailModal, { props: { card: baseCard }, global: { stubs } });
    await wrapper.trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('emits "close" when clicking the close button', async () => {
    wrapper = mount(CardDetailModal, { props: { card: baseCard }, global: { stubs } });
    await wrapper.find('button').trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('does not emit "close" when clicking inside the panel', async () => {
    wrapper = mount(CardDetailModal, { props: { card: baseCard }, global: { stubs } });
    await wrapper.find('h3').trigger('click');
    expect(wrapper.emitted('close')).toBeUndefined();
  });

  it('emits "close" on Escape key while mounted', async () => {
    wrapper = mount(CardDetailModal, { props: { card: baseCard }, global: { stubs } });
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('stops listening for Escape after unmount', async () => {
    wrapper = mount(CardDetailModal, { props: { card: baseCard }, global: { stubs } });
    wrapper.unmount();
    expect(() =>
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })),
    ).not.toThrow();
  });
});
