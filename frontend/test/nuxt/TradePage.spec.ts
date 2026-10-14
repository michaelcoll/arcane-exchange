import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mockNuxtImport, mountSuspended } from '@nuxt/test-utils/runtime';
import type { Ref } from 'vue';
import type { DOMWrapper } from '@vue/test-utils';
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeDetail } from '~/bindings/TradeDetail';
import TradePage from '~/pages/trade/[id].vue';
import formatPrice from '~/utils/format-price';

// `useAuth` is also called by app.vue and the register-user plugin while the Nuxt test app
// boots, so the mocked state must hold refs from the start (as in `PlayerPicker.spec`).
const { authState } = await vi.hoisted(async () => {
  const { ref } = await import('vue');
  return {
    authState: {
      isLoaded: ref(true) as Ref<boolean>,
      isSignedIn: ref<boolean | undefined>(true),
      getToken: ref(vi.fn().mockResolvedValue('token')),
    },
  };
});

const { tradeService } = vi.hoisted(() => ({
  tradeService: {
    getTrade: vi.fn(),
    removeCard: vi.fn(),
    acceptTrade: vi.fn(),
    abandonTrade: vi.fn(),
    confirmTrade: vi.fn(),
    rateTrade: vi.fn(),
  },
}));

mockNuxtImport('useAuth', () => () => authState);
mockNuxtImport('useTradeService', () => () => tradeService);
// The sides render a `PlayerAvatar`, which has no backend to call here.
mockNuxtImport('useUserService', () => () => ({
  register: () => Promise.resolve(),
  getUserProfile: () => Promise.resolve({ id: 'user', username: 'user', avatar_url: null }),
}));

const card = (name: string, trend: number): TradeCard => ({
  set_code: 'ECL',
  collector_number: name,
  language_code: 'FR',
  foil: false,
  name,
  quantity: 1,
  price_guide: { low: 0, avg: 0, trend },
  scryfall_id: name,
  image_url: null,
  image_back_url: null,
});

const party = { accepted: false, confirmed: false, rating: null };

/** A Trade where I give a Sol Ring (12 €) and receive a Vampiric Tutor (33 €). */
const tradeIn = (status: string, overrides: Partial<TradeDetail> = {}): TradeDetail => ({
  id: 't-1',
  status,
  partner_username: 'mizzix_42',
  my_cards: [card('Sol Ring', 1200)],
  partner_cards: [card('Vampiric Tutor', 3300)],
  me: party,
  partner: party,
  ...overrides,
});

const mountTrade = async (detail: TradeDetail) => {
  tradeService.getTrade.mockResolvedValue(detail);
  // The card detail modal loads prices and offers: only its opening is in scope here.
  const wrapper = await mountSuspended(TradePage, {
    route: '/trade/t-1',
    global: { stubs: { CardDetailModal: true } },
  });
  await vi.waitFor(() => expect(wrapper.find('[data-part="desktop"]').exists()).toBe(true));
  return wrapper;
};

/** The headers of the sides of a region, in document order. */
const sideHeaders = (region: Pick<DOMWrapper<Element>, 'findAll'>) =>
  region.findAll('[data-part="header"]').map((h) => h.text());

describe('Trade page', () => {
  beforeEach(() => {
    for (const fn of Object.values(tradeService)) fn.mockReset();
  });

  it('puts the partner side first, then mine, each with its total, on desktop', async () => {
    const wrapper = await mountTrade(tradeIn('PENDING'));
    const headers = sideHeaders(wrapper.get('[data-part="desktop"]'));

    expect(headers).toHaveLength(2);
    expect(headers[0]).toContain('@MIZZIX_42');
    expect(headers[0]).toContain(formatPrice(3300));
    expect(headers[1]).toContain('JE POSE');
    expect(headers[1]).toContain(formatPrice(1200));
  });

  it('heads the desktop region with the partner and the step dots', async () => {
    const wrapper = await mountTrade(tradeIn('PENDING'));
    const desktop = wrapper.get('[data-part="desktop"]');

    expect(desktop.find('button[aria-label="Retour"]').exists()).toBe(true);
    expect(desktop.text()).toContain('@mizzix_42');
    expect(desktop.text()).toContain('NÉGOCIATION · 1/5');
  });

  it('shows no status banner in either region', async () => {
    const wrapper = await mountTrade(tradeIn('PENDING'));

    expect(wrapper.text()).not.toContain('Négociation ouverte');
  });

  it.each(['mobile', 'desktop'])(
    'opens the detail modal of a card clicked on %s, and closes it',
    async (region) => {
      const wrapper = await mountTrade(tradeIn('PENDING'));
      const modal = () => wrapper.findComponent({ name: 'CardDetailModal' });
      expect(modal().exists()).toBe(false);

      await wrapper
        .get(`[data-part="${region}"]`)
        .get('[data-part="face"] [title="Vampiric Tutor"]')
        .trigger('click');
      expect(modal().props('card')).toMatchObject({ name: 'Vampiric Tutor' });

      modal().vm.$emit('close');
      await wrapper.vm.$nextTick();
      expect(modal().exists()).toBe(false);
    },
  );

  it('opens the steps sheet from the (i) of the desktop header, and closes it', async () => {
    const wrapper = await mountTrade(tradeIn('PENDING'));
    const sheet = () => wrapper.find('[role="dialog"][aria-labelledby="trade-steps-title"]');
    expect(sheet().exists()).toBe(false);

    await wrapper
      .get('[data-part="desktop"]')
      .get('button[aria-label="Les étapes d\'un échange"]')
      .trigger('click');
    expect(sheet().exists()).toBe(true);

    await sheet().get('button[aria-label="Fermer"]').trigger('click');
    expect(sheet().exists()).toBe(false);
  });

  it('lets me remove the cards I receive, never mine, while the Trade is editable', async () => {
    const wrapper = await mountTrade(tradeIn('PENDING'));
    const desktop = wrapper.get('[data-part="desktop"]');

    const remove = desktop.findAll('button[aria-label^="Retirer "]');
    expect(remove.map((b) => b.attributes('aria-label'))).toEqual(['Retirer Vampiric Tutor']);
    expect(desktop.get('[data-part="add"]').text()).toContain('Chercher chez lui');
  });

  it('lets me remove nothing nor add anything once the Trade is locked', async () => {
    const wrapper = await mountTrade(tradeIn('FULLY_ACCEPTED'));
    const desktop = wrapper.get('[data-part="desktop"]');

    expect(desktop.find('button[aria-label^="Retirer "]').exists()).toBe(false);
    expect(desktop.find('[data-part="add"]').exists()).toBe(false);
    expect(desktop.findAll('[aria-label="Carte réservée"]')).toHaveLength(2);
    expect(desktop.text()).toContain('échange réservé');
  });

  it('says the delta cash only in the accept button, without a balance column', async () => {
    const wrapper = await mountTrade(tradeIn('PENDING'));
    const desktop = wrapper.get('[data-part="desktop"]');

    expect(desktop.text()).not.toContain('Réglé hors plateforme');
    const accept = desktop.findAll('button').find((b) => b.text().startsWith('Accepter'));
    expect(accept?.text()).toBe(`Accepter et payer ${formatPrice(2100)}`);
  });

  describe('abandon ×', () => {
    const abandon = 'button[aria-label="Abandonner l\'échange"]';
    const confirmation = '[role="dialog"][aria-modal="true"]';

    it.each(['PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED'])(
      'is offered on desktop only while %s',
      async (status) => {
        const wrapper = await mountTrade(tradeIn(status));

        expect(wrapper.get('[data-part="desktop"]').find(abandon).exists()).toBe(true);
        expect(wrapper.get('[data-part="mobile"]').find(abandon).exists()).toBe(false);
      },
    );

    it.each(['COMPLETED', 'CLOSED', 'ABANDONED'])('is not offered once %s', async (status) => {
      const wrapper = await mountTrade(tradeIn(status));
      expect(wrapper.find(abandon).exists()).toBe(false);
    });

    it('abandons the Trade once its confirmation is accepted', async () => {
      tradeService.abandonTrade.mockResolvedValue(undefined);
      const wrapper = await mountTrade(tradeIn('PENDING'));

      await wrapper.get('[data-part="desktop"]').get(abandon).trigger('click');
      const dialog = wrapper.get(confirmation);
      expect(dialog.text()).toContain("Abandonner l'échange ?");
      expect(tradeService.abandonTrade).not.toHaveBeenCalled();

      const confirm = dialog.findAll('button').find((b) => b.text() === 'Abandonner');
      await confirm?.trigger('click');
      expect(tradeService.abandonTrade).toHaveBeenCalledWith('t-1');
    });
  });

  it.each(['COMPLETED', 'CLOSED'])('shows the rating on desktop once %s', async (status) => {
    const wrapper = await mountTrade(tradeIn(status));
    const desktop = wrapper.get('[data-part="desktop"]');

    expect(desktop.find('section[aria-labelledby="trade-rating-title"]').exists()).toBe(true);
    expect(desktop.text()).toContain('Noter @mizzix_42');
  });
});
