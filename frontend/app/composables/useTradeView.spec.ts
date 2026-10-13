import { describe, it, expect, vi } from 'vitest';
import { ref } from 'vue';
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeDetail } from '~/bindings/TradeDetail';
import type { TradePartyState } from '~/bindings/TradePartyState';
import type { TradeStatusParam } from '~/bindings/TradeStatusParam';
import formatPrice from '../utils/format-price';
import { useTradeView, type TradeViewServices } from './useTradeView';

const card = (overrides: Partial<TradeCard> = {}): TradeCard => ({
  set_code: 'ECL',
  collector_number: '166',
  language_code: 'FR',
  foil: false,
  name: 'Sol Ring',
  quantity: 1,
  price_guide: { low: 100, avg: 150, trend: 200 },
  scryfall_id: 'abc',
  image_url: null,
  image_back_url: null,
  ...overrides,
});

/** A card of a given value, in cents. */
const worth = (cents: number, overrides: Partial<TradeCard> = {}) =>
  card({ price_guide: { low: cents, avg: cents, trend: cents }, ...overrides });

const party = (overrides: Partial<TradePartyState> = {}): TradePartyState => ({
  accepted: false,
  confirmed: false,
  rating: null,
  ...overrides,
});

const makeTrade = (overrides: Partial<TradeDetail> = {}): TradeDetail => ({
  id: 'trade-1',
  status: 'PENDING',
  partner_username: 'jace',
  my_cards: [],
  partner_cards: [],
  me: party(),
  partner: party(),
  ...overrides,
});

const makeServices = (overrides: Partial<TradeViewServices> = {}): TradeViewServices => ({
  accept: vi.fn().mockResolvedValue(undefined),
  abandon: vi.fn().mockResolvedValue(undefined),
  confirmExchange: vi.fn().mockResolvedValue(undefined),
  rate: vi.fn().mockResolvedValue(undefined),
  removeCard: vi.fn().mockResolvedValue(undefined),
  reload: vi.fn().mockResolvedValue(undefined),
  reportRefusal: vi.fn(),
  ...overrides,
});

/** The view of an already loaded Trade, with its fake services. */
const setup = (
  tradeOverrides: Partial<TradeDetail> = {},
  serviceOverrides: Partial<TradeViewServices> = {},
) => {
  const trade = ref<TradeDetail | undefined>(makeTrade(tradeOverrides));
  const services = makeServices(serviceOverrides);
  const tradeView = useTradeView(trade, services);
  // Throws rather than returning `null`: every test reads the view of a loaded Trade.
  const view = () => {
    if (!tradeView.view.value) throw new Error('the Trade view is not loaded');
    return tradeView.view.value;
  };
  return { ...tradeView, trade, services, view };
};

const ALL_STATUSES: TradeStatusParam[] = [
  'PENDING',
  'ONE_ACCEPTED',
  'FULLY_ACCEPTED',
  'COMPLETED',
  'CLOSED',
  'ABANDONED',
];

describe('useTradeView', () => {
  describe("l'action principale", () => {
    it.each(['PENDING', 'ONE_ACCEPTED'] as const)(
      "propose d'accepter en %s tant que je n'ai pas accepté",
      (status) => {
        const { view } = setup({ status });

        expect(view().primaryAction).toEqual({ kind: 'accept', label: "Accepter l'échange" });
      },
    );

    it("porte le règlement sur le libellé d'acceptation", () => {
      const { view } = setup({ my_cards: [worth(400)], partner_cards: [worth(2500)] });

      expect(view().primaryAction).toEqual({
        kind: 'accept',
        label: `Accepter et payer ${formatPrice(2100)}`,
      });
    });

    it.each(['PENDING', 'ONE_ACCEPTED'] as const)(
      "attend l'acceptation du partenaire en %s une fois que j'ai accepté",
      (status) => {
        const { view } = setup({ status, me: party({ accepted: true }) });

        expect(view().primaryAction).toEqual({ kind: 'awaitingAcceptance' });
      },
    );

    it("propose de confirmer l'échange réalisé en FULLY_ACCEPTED", () => {
      const { view } = setup({ status: 'FULLY_ACCEPTED', me: party({ accepted: true }) });

      expect(view().primaryAction).toEqual({ kind: 'confirmExchange' });
    });

    it("attend la confirmation du partenaire en FULLY_ACCEPTED une fois que j'ai confirmé", () => {
      const { view } = setup({
        status: 'FULLY_ACCEPTED',
        me: party({ accepted: true, confirmed: true }),
      });

      expect(view().primaryAction).toEqual({ kind: 'awaitingConfirmation' });
    });

    it.each(['COMPLETED', 'CLOSED', 'ABANDONED'] as const)(
      "n'offre aucune action en %s",
      (status) => {
        const { view } = setup({ status, me: party({ accepted: true, confirmed: true }) });

        expect(view().primaryAction).toEqual({ kind: 'none' });
      },
    );
  });

  describe('les drapeaux', () => {
    it.each([
      { status: 'PENDING', editable: true, reserved: false, abandonable: true },
      { status: 'ONE_ACCEPTED', editable: true, reserved: true, abandonable: true },
      { status: 'FULLY_ACCEPTED', editable: false, reserved: true, abandonable: true },
      { status: 'COMPLETED', editable: false, reserved: false, abandonable: false },
      { status: 'CLOSED', editable: false, reserved: false, abandonable: false },
      { status: 'ABANDONED', editable: false, reserved: false, abandonable: false },
    ] as const)(
      'en $status : modifiable $editable, réservé $reserved, abandonnable $abandonable',
      ({ status, editable, reserved, abandonable }) => {
        const { view } = setup({ status });

        expect(view().status).toBe(status);
        expect(view().editable).toBe(editable);
        expect(view().reserved).toBe(reserved);
        expect(view().abandonable).toBe(abandonable);
      },
    );

    it('replie un statut inconnu sur PENDING', () => {
      const { view } = setup({ status: 'SOMETHING_NEW' });

      expect(view().status).toBe('PENDING');
    });
  });

  describe('les parties', () => {
    it('expose moi et le partenaire avec acceptation, confirmation et Note', () => {
      const { view } = setup({
        status: 'COMPLETED',
        me: party({ accepted: true, confirmed: true, rating: 5 }),
        partner: party({ accepted: true, confirmed: true, rating: null }),
      });

      expect(view().partnerUsername).toBe('jace');
      expect(view().me).toEqual({ accepted: true, confirmed: true, rating: 5 });
      expect(view().partner).toEqual({ accepted: true, confirmed: true, rating: null });
    });
  });

  describe('les côtés et le solde', () => {
    it('totalise ce que je donne et ce que je reçois, quantités comprises', () => {
      const given = [worth(400)];
      const received = [worth(1000, { quantity: 2 }), worth(500, { collector_number: '7' })];
      const { view } = setup({ my_cards: given, partner_cards: received });

      expect(view().give).toEqual({ cards: given, total: 400 });
      expect(view().receive).toEqual({ cards: received, total: 2500 });
    });

    it('compte pour zéro une carte sans prix', () => {
      const { view } = setup({ partner_cards: [card({ price_guide: null }), worth(700)] });

      expect(view().receive.total).toBe(700);
    });

    it('me fait payer quand je reçois plus que je ne donne', () => {
      const { view } = setup({ my_cards: [worth(400)], partner_cards: [worth(2500)] });

      expect(view().balance).toEqual({
        delta: 2100,
        balanced: false,
        settlement: `payer ${formatPrice(2100)}`,
      });
    });

    it('me fait recevoir quand je donne plus que je ne reçois', () => {
      const { view } = setup({ my_cards: [worth(1000)], partner_cards: [worth(600)] });

      expect(view().balance).toEqual({
        delta: -400,
        balanced: false,
        settlement: `recevoir ${formatPrice(400)}`,
      });
    });

    it('tient le Trade pour équilibré sous 3 € d’écart, sans règlement', () => {
      const { view } = setup({ my_cards: [worth(1000)], partner_cards: [worth(1299)] });

      expect(view().balance).toEqual({ delta: 299, balanced: true, settlement: null });
      expect(view().primaryAction).toEqual({ kind: 'accept', label: "Accepter l'échange" });
    });

    it('appelle un règlement dès 3 € d’écart', () => {
      const { view } = setup({ my_cards: [worth(1300)], partner_cards: [worth(1000)] });

      expect(view().balance.balanced).toBe(false);
      expect(view().balance.settlement).toBe(`recevoir ${formatPrice(300)}`);
    });
  });

  describe('la notation', () => {
    it.each(['PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED', 'ABANDONED'] as const)(
      "n'est pas visible en %s",
      (status) => {
        const { view } = setup({ status });

        expect(view().rating.visible).toBe(false);
        expect(view().rating.canRate).toBe(false);
      },
    );

    it("est visible et possible en COMPLETED tant que je n'ai pas noté", () => {
      const { view } = setup({ status: 'COMPLETED' });

      expect(view().rating).toEqual({ visible: true, canRate: true, mine: null, partner: null });
    });

    it("n'est plus possible une fois ma Note posée", () => {
      const { view } = setup({ status: 'COMPLETED', me: party({ rating: 4 }) });

      expect(view().rating).toMatchObject({ visible: true, canRate: false, mine: 4 });
    });

    it('garde une notation passée comme une Note 0, non modifiable', () => {
      const { view } = setup({ status: 'COMPLETED', me: party({ rating: 0 }) });

      expect(view().rating).toMatchObject({ visible: true, canRate: false, mine: 0 });
    });

    it('expose la Note du partenaire dès qu’elle existe', () => {
      const { view } = setup({ status: 'COMPLETED', partner: party({ rating: 3 }) });

      expect(view().rating).toMatchObject({ canRate: true, partner: 3 });
    });

    it('reste visible en CLOSED avec les deux Notes, sans pouvoir noter', () => {
      const { view } = setup({
        status: 'CLOSED',
        me: party({ rating: 5 }),
        partner: party({ rating: 0 }),
      });

      expect(view().rating).toEqual({ visible: true, canRate: false, mine: 5, partner: 0 });
    });
  });

  describe('les étapes', () => {
    it('nomme les cinq étapes du parcours nominal, dans leur ordre', () => {
      const { view } = setup();

      expect(view().steps.list.map((step) => step.title)).toEqual([
        'Négociation',
        '1 acceptation',
        'Verrouillé',
        'Échange réalisé',
        'Clôturé',
      ]);
    });

    it.each([
      ['PENDING', 0, ['current', 'upcoming', 'upcoming', 'upcoming', 'upcoming']],
      ['ONE_ACCEPTED', 1, ['done', 'current', 'upcoming', 'upcoming', 'upcoming']],
      ['FULLY_ACCEPTED', 2, ['done', 'done', 'current', 'upcoming', 'upcoming']],
      ['COMPLETED', 3, ['done', 'done', 'done', 'current', 'upcoming']],
      ['CLOSED', 4, ['done', 'done', 'done', 'done', 'current']],
    ] as const)(
      'franchit les étapes précédentes et met en cours celle de %s',
      (status, index, states) => {
        const { view } = setup({ status });

        expect(view().steps.currentIndex).toBe(index);
        expect(view().steps.list.map((step) => step.state)).toEqual(states);
      },
    );

    it("n'a aucune étape en cours en ABANDONED, hors parcours", () => {
      const { view } = setup({ status: 'ABANDONED' });

      expect(view().steps.currentIndex).toBeNull();
      expect(view().steps.list.map((step) => step.state)).toEqual([
        'upcoming',
        'upcoming',
        'upcoming',
        'upcoming',
        'upcoming',
      ]);
    });
  });

  describe('un Trade pas encore chargé', () => {
    it("n'a pas de vue, puis en a une dès que le Trade arrive", () => {
      const trade = ref<TradeDetail | undefined>(undefined);
      const { view } = useTradeView(trade, makeServices());

      expect(view.value).toBeNull();

      trade.value = makeTrade({ status: 'FULLY_ACCEPTED' });

      expect(view.value?.status).toBe('FULLY_ACCEPTED');
    });
  });

  describe('les confirmations', () => {
    it("demande une confirmation avant d'accepter, en rappelant le règlement", () => {
      const { view, requestAccept, services } = setup({
        my_cards: [worth(400)],
        partner_cards: [worth(2500)],
      });

      requestAccept();

      expect(view().confirmation).toMatchObject({
        kind: 'accept',
        title: 'Accepter cet échange ?',
        confirmLabel: 'Accepter',
        tone: 'primary',
      });
      expect(view().confirmation?.body).toContain(
        `Tu t'engages à payer ${formatPrice(2100)} en main propre.`,
      );
      expect(services.accept).not.toHaveBeenCalled();
    });

    it("accepte quand la confirmation d'acceptation est validée", async () => {
      const { view, requestAccept, confirmPending, services } = setup();

      requestAccept();
      await confirmPending();

      expect(services.accept).toHaveBeenCalledTimes(1);
      expect(view().confirmation).toBeNull();
    });

    it("demande une confirmation avant d'abandonner", () => {
      const { view, requestAbandon, services } = setup({ status: 'FULLY_ACCEPTED' });

      requestAbandon();

      expect(view().confirmation).toMatchObject({
        kind: 'abandon',
        title: "Abandonner l'échange ?",
        confirmLabel: 'Abandonner',
        tone: 'down',
      });
      expect(services.abandon).not.toHaveBeenCalled();
    });

    it("abandonne quand la confirmation d'abandon est validée", async () => {
      const { requestAbandon, confirmPending, services } = setup();

      requestAbandon();
      await confirmPending();

      expect(services.abandon).toHaveBeenCalledTimes(1);
      expect(services.accept).not.toHaveBeenCalled();
    });

    it.each([
      { name: 'acceptation', ask: 'requestAccept' },
      { name: 'abandon', ask: 'requestAbandon' },
    ] as const)("n'appelle aucun service quand l'$name est annulée", async ({ ask }) => {
      const ctx = setup();

      ctx[ask]();
      ctx.cancelPending();
      // Validating afterwards must not run anything either: the confirmation no longer exists.
      await ctx.confirmPending();

      expect(ctx.view().confirmation).toBeNull();
      for (const service of Object.values(ctx.services)) expect(service).not.toHaveBeenCalled();
    });

    it('ne fait rien à la validation quand aucune confirmation n’est ouverte', async () => {
      const { confirmPending, services } = setup();

      await confirmPending();

      for (const service of Object.values(services)) expect(service).not.toHaveBeenCalled();
    });
  });

  describe("le retrait d'une carte reçue", () => {
    it('est immédiat en PENDING', async () => {
      const wanted = worth(700);
      const { view, removeReceivedCard, services } = setup({ partner_cards: [wanted] });

      await removeReceivedCard(wanted);

      expect(services.removeCard).toHaveBeenCalledExactlyOnceWith(wanted);
      expect(view().confirmation).toBeNull();
    });

    it('passe par la confirmation « modifier » en ONE_ACCEPTED', async () => {
      const wanted = worth(700);
      const { view, removeReceivedCard, confirmPending, services } = setup({
        status: 'ONE_ACCEPTED',
        partner_cards: [wanted],
      });

      await removeReceivedCard(wanted);

      expect(view().confirmation).toMatchObject({
        kind: 'modify',
        title: "Modifier l'échange ?",
        confirmLabel: 'Modifier quand même',
        tone: 'down',
      });
      expect(services.removeCard).not.toHaveBeenCalled();

      await confirmPending();

      expect(services.removeCard).toHaveBeenCalledExactlyOnceWith(wanted);
      expect(view().confirmation).toBeNull();
    });

    it('ne retire rien quand la confirmation « modifier » est annulée', async () => {
      const wanted = worth(700);
      const { removeReceivedCard, cancelPending, services } = setup({
        status: 'ONE_ACCEPTED',
        partner_cards: [wanted],
      });

      await removeReceivedCard(wanted);
      cancelPending();

      for (const service of Object.values(services)) expect(service).not.toHaveBeenCalled();
    });

    it.each(['FULLY_ACCEPTED', 'COMPLETED', 'CLOSED', 'ABANDONED'] as const)(
      "ne retire rien en %s, le Trade n'étant plus modifiable",
      async (status) => {
        const wanted = worth(700);
        const { view, removeReceivedCard, services } = setup({ status, partner_cards: [wanted] });

        await removeReceivedCard(wanted);

        expect(view().confirmation).toBeNull();
        for (const service of Object.values(services)) expect(service).not.toHaveBeenCalled();
      },
    );
  });

  describe('les actions sans confirmation', () => {
    it("confirme l'échange réalisé directement", async () => {
      const { view, confirmExchange, services } = setup({ status: 'FULLY_ACCEPTED' });

      await confirmExchange();

      expect(services.confirmExchange).toHaveBeenCalledTimes(1);
      expect(view().confirmation).toBeNull();
    });

    it('note le partenaire directement', async () => {
      const { rate, services } = setup({ status: 'COMPLETED' });

      await rate(4);

      expect(services.rate).toHaveBeenCalledExactlyOnceWith(4);
    });

    it('enregistre une notation passée comme une Note 0', async () => {
      const { skipRating, services } = setup({ status: 'COMPLETED' });

      await skipRating();

      expect(services.rate).toHaveBeenCalledExactlyOnceWith(0);
    });
  });

  describe("le cycle d'une action", () => {
    /** A promise the test resolves or rejects itself. */
    const deferred = () => {
      let resolve!: () => void;
      let reject!: (reason: unknown) => void;
      const promise = new Promise<void>((res, rej) => {
        resolve = res;
        reject = rej;
      });
      return { promise, resolve, reject };
    };

    it("est occupée pendant l'action et son rechargement, puis ne l'est plus", async () => {
      const action = deferred();
      const reloading = deferred();
      const { view, confirmExchange, services } = setup(
        { status: 'FULLY_ACCEPTED' },
        {
          confirmExchange: vi.fn(() => action.promise),
          reload: vi.fn(() => reloading.promise),
        },
      );

      const done = confirmExchange();

      expect(view().busy).toBe(true);
      expect(services.reload).not.toHaveBeenCalled();

      action.resolve();
      await vi.waitFor(() => expect(services.reload).toHaveBeenCalledTimes(1));
      expect(view().busy).toBe(true);

      reloading.resolve();
      await done;

      expect(view().busy).toBe(false);
      expect(services.reportRefusal).not.toHaveBeenCalled();
    });

    it("n'appelle pas un service une seconde fois tant qu'une action est en cours", async () => {
      const action = deferred();
      const { view, confirmExchange, rate, services } = setup(
        { status: 'FULLY_ACCEPTED' },
        { confirmExchange: vi.fn(() => action.promise) },
      );

      const first = confirmExchange();
      const second = confirmExchange();
      const other = rate(4);

      action.resolve();
      await Promise.all([first, second, other]);

      expect(services.confirmExchange).toHaveBeenCalledTimes(1);
      expect(services.rate).not.toHaveBeenCalled();
      expect(services.reload).toHaveBeenCalledTimes(1);
      expect(view().busy).toBe(false);
    });

    it('montre le Trade rechargé après une action réussie', async () => {
      const ctx = setup({ status: 'FULLY_ACCEPTED', me: party({ accepted: true }) });
      vi.mocked(ctx.services.reload).mockImplementation(async () => {
        ctx.trade.value = makeTrade({
          status: 'FULLY_ACCEPTED',
          me: party({ accepted: true, confirmed: true }),
        });
      });

      await ctx.confirmExchange();

      expect(ctx.view().primaryAction).toEqual({ kind: 'awaitingConfirmation' });
    });

    it("signale un refus avec le message de l'API, puis recharge", async () => {
      const refused = {
        statusCode: 409,
        data: { code: 'trade_already_accepted', error: 'trade already accepted' },
      };
      const { view, requestAccept, confirmPending, services } = setup(
        {},
        { accept: vi.fn().mockRejectedValue(refused) },
      );

      requestAccept();
      await confirmPending();

      expect(services.reportRefusal).toHaveBeenCalledExactlyOnceWith(
        'Tu as déjà accepté cet échange.',
      );
      expect(services.reload).toHaveBeenCalledTimes(1);
      expect(view().busy).toBe(false);
    });

    it('signale un échec sans réponse de l’API avec le message générique, puis recharge', async () => {
      const { view, rate, services } = setup(
        { status: 'COMPLETED' },
        { rate: vi.fn().mockRejectedValue(new TypeError('Failed to fetch')) },
      );

      await rate(5);

      expect(services.reportRefusal).toHaveBeenCalledExactlyOnceWith('Une erreur est survenue.');
      expect(services.reload).toHaveBeenCalledTimes(1);
      expect(view().busy).toBe(false);
    });
  });

  describe('les intentions sur un Trade pas encore chargé', () => {
    it("n'ouvrent aucune confirmation et n'appellent aucun service", async () => {
      const trade = ref<TradeDetail | undefined>(undefined);
      const services = makeServices();
      const tradeView = useTradeView(trade, services);

      tradeView.requestAccept();
      tradeView.requestAbandon();
      await tradeView.confirmPending();
      await tradeView.removeReceivedCard(worth(700));
      await tradeView.confirmExchange();
      await tradeView.rate(3);

      expect(tradeView.view.value).toBeNull();
      for (const service of Object.values(services)) expect(service).not.toHaveBeenCalled();

      // A confirmation requested before the load does not come back with the Trade.
      trade.value = makeTrade();
      expect(tradeView.view.value?.confirmation).toBeNull();
    });
  });

  it.each(ALL_STATUSES)('rend une vue au repos en %s', (status) => {
    const { view } = setup({ status });

    expect(view().busy).toBe(false);
    expect(view().confirmation).toBeNull();
  });
});
