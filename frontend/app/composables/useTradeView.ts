import { computed, ref, type Ref } from 'vue';
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeDetail } from '~/bindings/TradeDetail';
import type { TradeStatusParam } from '~/bindings/TradeStatusParam';
import { readApiError } from '../utils/api-error';
import {
  isTradeAbandonable,
  isTradeBalanced,
  isTradeEditable,
  isTradeReserved,
  toTradeStatus,
  TRADE_STATUS_META,
  TRADE_STEPS,
  tradeAcceptLabel,
  tradeCardsTotal,
  tradeConfirmation,
  tradeSettlementLabel,
  type TradeConfirmationKind,
  type TradeRating,
  type TradeStatusPresentation,
  type TradeTone,
} from '../utils/trade';

/** What the view asks of the page: the calls already target the displayed Trade. */
export interface TradeViewServices {
  accept: () => Promise<unknown>;
  abandon: () => Promise<unknown>;
  confirmExchange: () => Promise<unknown>;
  /** Rating from 1 to 5, 0 for a skipped rating. */
  rate: (rating: number) => Promise<unknown>;
  /** Removes a card I receive. */
  removeCard: (card: TradeCard) => Promise<unknown>;
  reload: () => Promise<unknown>;
  /** Reports a refused action, with the message to show the player. */
  reportRefusal: (message: string) => void;
}

/** A party of the Trade, me or the partner. */
export interface TradeViewParty {
  accepted: boolean;
  confirmed: boolean;
  rating: TradeRating;
}

/** A side of the Trade (what I give, what I receive), already totalled. `total` in cents. */
export interface TradeViewSide {
  cards: TradeCard[];
  total: number;
}

export interface TradeViewBalance {
  /** Delta cash: total received − total given, in cents. */
  delta: number;
  balanced: boolean;
  /** « payer 21 € », « recevoir 4 € », `null` when the Trade is balanced. */
  settlement: string | null;
}

/** The action offered to the player: a single one per status. */
export type TradePrimaryAction =
  | { kind: 'accept'; label: string }
  | { kind: 'awaitingAcceptance' }
  | { kind: 'confirmExchange' }
  | { kind: 'awaitingConfirmation' }
  | { kind: 'none' };

export interface TradeViewRating {
  /** The rating is shown in `COMPLETED` and `CLOSED`. */
  visible: boolean;
  /** I can still rate or skip the rating. */
  canRate: boolean;
  mine: TradeRating;
  partner: TradeRating;
}

export type TradeStepState = 'done' | 'current' | 'upcoming';

/** A step of the nominal path, located relative to the status of the Trade. */
export interface TradeViewStep {
  status: TradeStatusParam;
  title: string;
  detail: string;
  state: TradeStepState;
}

/** The five steps of the nominal path, the same for the desktop stepper, the dot indicator and
 * the « Les étapes d'un échange » sheet. */
export interface TradeViewSteps {
  list: TradeViewStep[];
  /** Position of the current step in `list`, `null` for an abandoned Trade: it is off the path,
   * no step is current. */
  currentIndex: number | null;
}

/** The status message: what is expected of me, given the status and what I already accepted or
 * confirmed. Its tone is the one of the status. */
export interface TradeViewBanner {
  title: string;
  text: string;
  icon: string;
  tone: TradeTone;
}

/** An open confirmation, awaiting the player's decision. */
export interface TradeViewConfirmation {
  kind: TradeConfirmationKind;
  title: string;
  body: string;
  confirmLabel: string;
  tone: 'primary' | 'down';
}

/** Everything the Trade screen tells the player, from the "me / partner" point of view (ADR 0005). */
export interface TradeView {
  status: TradeStatusParam;
  /** The label and the tone of the status. */
  presentation: TradeStatusPresentation;
  partnerUsername: string;
  me: TradeViewParty;
  partner: TradeViewParty;
  give: TradeViewSide;
  receive: TradeViewSide;
  balance: TradeViewBalance;
  /** Cards can still be added or removed. */
  editable: boolean;
  /** The cards of both sides are reserved. */
  reserved: boolean;
  abandonable: boolean;
  primaryAction: TradePrimaryAction;
  rating: TradeViewRating;
  steps: TradeViewSteps;
  banner: TradeViewBanner;
  /** An action is running: the actions are to be disabled. */
  busy: boolean;
  confirmation: TradeViewConfirmation | null;
}

/** `modify` keeps the card whose removal awaits the player's decision. */
type PendingConfirmation =
  { kind: 'accept' } | { kind: 'abandon' } | { kind: 'modify'; card: TradeCard };

const primaryActionOf = (
  status: TradeStatusParam,
  me: TradeViewParty,
  delta: number,
): TradePrimaryAction => {
  switch (status) {
    case 'PENDING':
    case 'ONE_ACCEPTED':
      return me.accepted
        ? { kind: 'awaitingAcceptance' }
        : { kind: 'accept', label: tradeAcceptLabel(delta) };
    case 'FULLY_ACCEPTED':
      return me.confirmed ? { kind: 'awaitingConfirmation' } : { kind: 'confirmExchange' };
    case 'COMPLETED':
    case 'CLOSED':
    case 'ABANDONED':
      return { kind: 'none' };
  }
};

const stepsOf = (status: TradeStatusParam): TradeViewSteps => {
  const position = TRADE_STEPS.findIndex((step) => step.status === status);
  const currentIndex = position < 0 ? null : position;
  const stateAt = (i: number): TradeStepState => {
    if (currentIndex === null || i > currentIndex) return 'upcoming';
    return i < currentIndex ? 'done' : 'current';
  };
  return {
    list: TRADE_STEPS.map((step, i) => ({ ...step, state: stateAt(i) })),
    currentIndex,
  };
};

/** Title, text and icon of the banner; `them` is the partner's username. */
const bannerMessageOf = (
  status: TradeStatusParam,
  me: TradeViewParty,
  them: string,
): Omit<TradeViewBanner, 'tone'> => {
  switch (status) {
    case 'PENDING':
      return {
        icon: 'lucide:arrow-left-right',
        title: 'Négociation ouverte',
        text: `Ajoute ou retire des cartes des deux côtés en composant avec ${them}.`,
      };
    case 'ONE_ACCEPTED':
      return me.accepted
        ? {
            icon: 'lucide:lock',
            title: `En attente de ${them}`,
            text: `Tu as accepté — les cartes sont réservées. ${them} doit accepter à son tour, ou tu peux encore modifier.`,
          }
        : {
            icon: 'lucide:lock',
            title: `${them} a accepté — à toi de jouer`,
            text: 'Accepte à ton tour pour verrouiller, ou modifie la transaction pour relancer la négociation.',
          };
    case 'FULLY_ACCEPTED':
      return {
        icon: 'lucide:lock',
        title: 'Transaction verrouillée',
        text: me.confirmed
          ? `Tu as confirmé l'échange. En attente de la confirmation de ${them}.`
          : 'Les deux parties ont accepté. Procédez à l’échange physique en personne, puis confirme ci-dessous.',
      };
    case 'COMPLETED':
      return {
        icon: 'lucide:check',
        title: 'Échange réalisé',
        text: 'Les deux parties ont confirmé l’échange physique. Tu peux noter ton partenaire (optionnel).',
      };
    case 'CLOSED':
      return {
        icon: 'lucide:check',
        title: 'Transaction clôturée',
        text: 'Les notations ont été enregistrées. Merci d’utiliser Arcane Exchange !',
      };
    case 'ABANDONED':
      return {
        icon: 'lucide:x',
        title: 'Échange abandonné',
        text: 'Cet échange a été abandonné. Les cartes réservées ont été libérées.',
      };
  }
};

/**
 * The player's view of the Trade: the only place that knows the status rules of the Trade screen.
 * It reads neither the route, the HTTP service nor the toast: the page hands it the loaded Trade
 * and wires the services.
 */
export const useTradeView = (
  trade: Readonly<Ref<TradeDetail | null | undefined>>,
  services: TradeViewServices,
) => {
  const busy = ref(false);
  const pending = ref<PendingConfirmation | null>(null);

  /** `null` until the Trade is loaded. */
  const view = computed<TradeView | null>(() => {
    const current = trade.value;
    if (!current) return null;

    const status = toTradeStatus(current.status);
    const give = { cards: current.my_cards, total: tradeCardsTotal(current.my_cards) };
    const receive = {
      cards: current.partner_cards,
      total: tradeCardsTotal(current.partner_cards),
    };
    const delta = receive.total - give.total;

    return {
      status,
      presentation: TRADE_STATUS_META[status],
      partnerUsername: current.partner_username,
      me: current.me,
      partner: current.partner,
      give,
      receive,
      balance: {
        delta,
        balanced: isTradeBalanced(delta),
        settlement: tradeSettlementLabel(delta),
      },
      editable: isTradeEditable(status),
      reserved: isTradeReserved(status),
      abandonable: isTradeAbandonable(status),
      primaryAction: primaryActionOf(status, current.me, delta),
      rating: {
        visible: status === 'COMPLETED' || status === 'CLOSED',
        canRate: status === 'COMPLETED' && current.me.rating == null,
        mine: current.me.rating,
        partner: current.partner.rating,
      },
      steps: stepsOf(status),
      banner: {
        ...bannerMessageOf(status, current.me, current.partner_username),
        tone: TRADE_STATUS_META[status].tone,
      },
      busy: busy.value,
      confirmation: pending.value && {
        kind: pending.value.kind,
        ...tradeConfirmation(pending.value.kind, delta),
      },
    };
  });

  /** Runs an action: busy until the end of the reload, which follows a success as well as a
   * failure so that the screen shows the real state of the Trade. An intent fired while an action
   * is running is dropped, so that a double trigger never calls a service twice. */
  const run = async (action: () => Promise<unknown>): Promise<void> => {
    if (busy.value) return;
    busy.value = true;
    try {
      await action();
    } catch (e) {
      services.reportRefusal(readApiError(e).message);
    } finally {
      try {
        await services.reload();
      } finally {
        busy.value = false;
      }
    }
  };

  /** Opens the acceptance confirmation. */
  const requestAccept = (): void => {
    if (view.value) pending.value = { kind: 'accept' };
  };

  /** Opens the abandon confirmation. */
  const requestAbandon = (): void => {
    if (view.value) pending.value = { kind: 'abandon' };
  };

  /** Validates the open confirmation and runs what it was guarding. */
  const confirmPending = async (): Promise<void> => {
    const current = pending.value;
    pending.value = null;
    if (!current) return;
    switch (current.kind) {
      case 'accept':
        return run(services.accept);
      case 'abandon':
        return run(services.abandon);
      case 'modify':
        return run(() => services.removeCard(current.card));
    }
  };

  /** Closes the open confirmation without running anything. */
  const cancelPending = (): void => {
    pending.value = null;
  };

  /** Removes a card I receive, only while the Trade is editable: immediate without any
   * acceptance, confirmed as soon as a party has accepted, since the removal then cancels the
   * acceptances. */
  const removeReceivedCard = async (card: TradeCard): Promise<void> => {
    if (!view.value?.editable) return;
    if (view.value.status === 'ONE_ACCEPTED') {
      pending.value = { kind: 'modify', card };
      return;
    }
    await run(() => services.removeCard(card));
  };

  /** Declares the physical exchange done. */
  const confirmExchange = async (): Promise<void> => {
    if (view.value) await run(services.confirmExchange);
  };

  /** Rates the partner, from 1 to 5. */
  const rate = async (rating: number): Promise<void> => {
    if (view.value) await run(() => services.rate(rating));
  };

  /** Skips the rating, recorded as a rating of 0. */
  const skipRating = (): Promise<void> => rate(0);

  return {
    view,
    requestAccept,
    requestAbandon,
    confirmPending,
    cancelPending,
    removeReceivedCard,
    confirmExchange,
    rate,
    skipRating,
  };
};
