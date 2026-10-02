import type { TradeCard } from '~/bindings/TradeCard';
import formatPrice from './format-price';

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

export type TradeTone = 'primary' | 'secondary' | 'good' | 'down' | 'muted';

/** Bordure, fond et texte d'une pastille (`Trade/StatusPill`, fraîcheur des prix du footer). */
export const TRADE_TONE_CLASSES: Record<TradeTone, string> = {
  primary: 'border-primary/30 bg-primary/10 text-primary-ink',
  secondary: 'border-secondary/30 bg-secondary/10 text-secondary-ink',
  good: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/30 dark:bg-emerald-400/10 dark:text-emerald-300',
  down: 'border-red-500/30 bg-red-500/10 text-red-600 dark:border-red-400/30 dark:bg-red-400/10 dark:text-red-400',
  muted:
    'border-slate-300 bg-slate-100 text-slate-500 dark:border-white/15 dark:bg-white/5 dark:text-slate-400',
};

/** Note laissée au partenaire : 1 à 5 étoiles, 0 si la notation a été passée, `null` tant que
 * non renseignée. */
export type TradeRating = number | null;

/** « Notation passée » pour une note 0 (voir **Note** dans CONTEXT.md), `n/5` sinon. */
export const formatTradeRating = (r: TradeRating) => {
  if (r == null) return 'non notée';
  return r === 0 ? 'Notation passée' : `${r}/5`;
};

/** Valeur d'une ligne de carte, en centimes : prix trend × quantité, 0 si le prix est inconnu. */
export const tradeCardValue = (card: TradeCard): number =>
  (card.price_guide?.trend ?? 0) * card.quantity;

/** Somme des valeurs de toutes les lignes, en centimes. */
export const tradeCardsTotal = (cards: TradeCard[]): number =>
  cards.reduce((s, c) => s + tradeCardValue(c), 0);

export const TRADE_STATUS_META: Record<TradeStatus, { label: string; tone: TradeTone }> = {
  PENDING: { label: 'En négociation', tone: 'primary' },
  ONE_ACCEPTED: { label: '1 acceptation', tone: 'primary' },
  FULLY_ACCEPTED: { label: 'Verrouillé', tone: 'secondary' },
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

/** L'échange peut encore être abandonné : à tout moment avant COMPLETED. */
export const isTradeAbandonable = (status: TradeStatus) =>
  isTradeEditable(status) || status === 'FULLY_ACCEPTED';

/** Les cinq étapes du parcours nominal, détaillées pour l'explication « Les étapes d'un
 * échange » — mêmes textes que `TradeSteps` côté iOS. */
export const TRADE_STEPS: { status: TradeStatus; title: string; detail: string }[] = [
  {
    status: 'PENDING',
    title: 'Négociation',
    detail:
      "Tu composes ta demande en piochant dans la collection de l'autre joueur ; lui compose la sienne dans la tienne. Chaque modification est notifiée.",
  },
  {
    status: 'ONE_ACCEPTED',
    title: '1 acceptation',
    detail:
      "Dès qu'un joueur accepte, les cartes des deux côtés sont réservées et les autres échanges qui les impliquent sont abandonnés.",
  },
  {
    status: 'FULLY_ACCEPTED',
    title: 'Verrouillé',
    detail:
      "Les deux ont accepté. Rendez-vous en main propre pour échanger les cartes et régler l'écart de valeur.",
  },
  {
    status: 'COMPLETED',
    title: 'Échange réalisé',
    detail:
      "Chacun confirme de son côté que l'échange a bien eu lieu. Les cartes changent alors de collection.",
  },
  {
    status: 'CLOSED',
    title: 'Clôturé',
    detail:
      "Vous pouvez vous noter mutuellement. Une fois les deux notes posées ou passées, l'échange est archivé.",
  },
];

/** Position du statut dans `TRADE_STEPS`, -1 pour ABANDONED (hors parcours nominal). */
export const tradeStepIndex = (status: TradeStatus) =>
  TRADE_STEPS.findIndex((s) => s.status === status);

/** Pastille de l'indicateur à points : « NÉGOCIATION · 1/5 », ou « ABANDONNÉ » hors parcours. */
export const tradeStatusStepLabel = (status: TradeStatus) => {
  const index = tradeStepIndex(status);
  if (index < 0) return 'ABANDONNÉ';
  return `${TRADE_STEPS[index]!.title} · ${index + 1}/${TRADE_STEPS.length}`.toUpperCase();
};

/** En dessous de 3 €, l'échange est considéré équilibré. */
const EVEN_THRESHOLD = 300;

/** Moitié « règlement » du bouton d'acceptation : « payer 21 € », « recevoir 4 € », `null` si
 * l'échange est équilibré. `diff` = total reçu − total donné, en centimes. */
export const tradeSettlementLabel = (diff: number): string | null => {
  if (Math.abs(diff) < EVEN_THRESHOLD) return null;
  const amount = formatPrice(Math.abs(diff));
  return diff > 0 ? `payer ${amount}` : `recevoir ${amount}`;
};

/** « Accepter et payer 21 € » : le montant fait partie de l'engagement, il va sur le bouton. */
export const tradeAcceptLabel = (diff: number) => {
  const settlement = tradeSettlementLabel(diff);
  return settlement ? `Accepter et ${settlement}` : "Accepter l'échange";
};

export type TradeConfirmationKind = 'accept' | 'abandon' | 'modify';

/** Textes des confirmations avant une étape verrouillante ou irréversible — identiques à
 * `TradeConfirmation` côté iOS. `diff` ne sert qu'à l'acceptation. */
export const tradeConfirmation = (
  kind: TradeConfirmationKind,
  diff: number,
): { title: string; body: string; confirmLabel: string; tone: 'primary' | 'down' } => {
  switch (kind) {
    case 'accept': {
      const settlement = tradeSettlementLabel(diff);
      const sentence = settlement
        ? `Tu t'engages à ${settlement} en main propre.`
        : 'Les valeurs sont équivalentes, aucun règlement.';
      return {
        title: 'Accepter cet échange ?',
        body: `${sentence} Les cartes des deux côtés seront réservées. Si l'autre partie modifie ensuite l'échange, il repassera en négociation et devra être accepté à nouveau.`,
        confirmLabel: 'Accepter',
        tone: 'primary',
      };
    }
    case 'abandon':
      return {
        title: "Abandonner l'échange ?",
        body: "L'échange sera définitivement abandonné et les cartes réservées libérées. Action irréversible.",
        confirmLabel: 'Abandonner',
        tone: 'down',
      };
    case 'modify':
      return {
        title: "Modifier l'échange ?",
        body: 'Une partie a déjà accepté. Modifier libère les cartes réservées, annule les acceptations et relance la négociation.',
        confirmLabel: 'Modifier quand même',
        tone: 'down',
      };
  }
};
