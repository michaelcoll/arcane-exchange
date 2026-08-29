import type { TradeCard } from '~/bindings/TradeCard';

/* Statuts de la machine à états d'un échange — miroir de `TradeStatus` côté backend
 * (src/ae/domain/trade.rs). Voir .agents/trade-workflow.instructions.md. */
export type TradeStatus =
  'PENDING' | 'ONE_ACCEPTED' | 'FULLY_ACCEPTED' | 'COMPLETED' | 'CLOSED' | 'ABANDONED';

const ALL_TRADE_STATUSES: TradeStatus[] = [
  'PENDING',
  'ONE_ACCEPTED',
  'FULLY_ACCEPTED',
  'COMPLETED',
  'CLOSED',
  'ABANDONED',
];

/** Valide et rétrécit le `status: string` du binding ts-rs vers l'union locale, repli sur `PENDING`. */
export const toTradeStatus = (raw: string): TradeStatus =>
  (ALL_TRADE_STATUSES as string[]).includes(raw) ? (raw as TradeStatus) : 'PENDING';

export type TradeTone = 'cyan' | 'violet' | 'good' | 'down' | 'muted';

/** Note laissée au partenaire : 1 à 5 étoiles, `null` tant que non renseignée. */
export type TradeRating = number | null;

/** Valeur d'une ligne de carte, en centimes : prix trend × quantité, 0 si le prix est inconnu. */
export const tradeCardValue = (card: TradeCard): number =>
  (card.price_guide?.trend ?? 0) * card.quantity;

/** Somme des valeurs de toutes les lignes, en centimes. */
export const tradeCardsTotal = (cards: TradeCard[]): number =>
  cards.reduce((s, c) => s + tradeCardValue(c), 0);

export const TRADE_STATUS_META: Record<TradeStatus, { label: string; tone: TradeTone }> = {
  PENDING: { label: 'En négociation', tone: 'cyan' },
  ONE_ACCEPTED: { label: '1 acceptation', tone: 'cyan' },
  FULLY_ACCEPTED: { label: 'Verrouillé', tone: 'violet' },
  COMPLETED: { label: 'Échange réalisé', tone: 'good' },
  CLOSED: { label: 'Clôturée', tone: 'muted' },
  ABANDONED: { label: 'Abandonnée', tone: 'down' },
};

/** Étapes du stepper de cycle de vie (ABANDONED est hors parcours nominal). */
export const TRADE_LIFECYCLE: { status: TradeStatus; label: string }[] = [
  { status: 'PENDING', label: 'Négociation' },
  { status: 'ONE_ACCEPTED', label: '1 acceptation' },
  { status: 'FULLY_ACCEPTED', label: 'Verrouillé' },
  { status: 'COMPLETED', label: 'Échange' },
  { status: 'CLOSED', label: 'Clôturé' },
];

/** L'échange peut encore être modifié (ajout/retrait de cartes). */
export const isTradeEditable = (status: TradeStatus) =>
  status === 'PENDING' || status === 'ONE_ACCEPTED';

/** Les cartes des deux côtés sont réservées. */
export const isTradeReserved = (status: TradeStatus) =>
  status === 'ONE_ACCEPTED' || status === 'FULLY_ACCEPTED';
