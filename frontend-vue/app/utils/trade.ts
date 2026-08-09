/* Statuts de la machine à états d'un échange — miroir de `TradeStatus` côté backend
 * (src/ccpt/domain/trade.rs). Voir .agents/trade-workflow.instructions.md. */
export type TradeStatus =
  'PENDING' | 'ONE_ACCEPTED' | 'FULLY_ACCEPTED' | 'COMPLETED' | 'CLOSED' | 'ABANDONED';

export type TradeTone = 'cyan' | 'violet' | 'good' | 'down' | 'muted';

/** Note laissée au partenaire : 1 à 5 étoiles, `skip` si passée, `null` tant que non renseignée. */
export type TradeRating = number | 'skip' | null;

/** Carte posée d'un côté de l'échange. `edh` = part des decks EDHREC qui la jouent. */
export interface TradeCard {
  name: string;
  eur: number;
  edh: number;
  scryfallId?: string;
  theGathererId?: string;
}

/** Mode de comparaison de valeur des deux colonnes. */
export type TradeValueMode = 'eur' | 'edh';

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
