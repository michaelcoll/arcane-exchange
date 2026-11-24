import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeStatusParam } from '~/bindings/TradeStatusParam';
import formatPrice from './format-price';

/* A Trade status is typed by the generated contract (`TradeStatusParam`, the only status union
 * ts-rs emits): a status added in the backend breaks the compilation of `TRADE_STATUS_META`. See
 * .claude/instructions/trade-workflow.instructions.md. */

/** Narrows the `status: string` of `TradeDetail` / `TradeSummary` to the contract status, falling
 * back to `PENDING`. */
export const toTradeStatus = (raw: string): TradeStatusParam =>
  Object.hasOwn(TRADE_STATUS_META, raw) ? (raw as TradeStatusParam) : 'PENDING';

export type TradeTone = 'primary' | 'secondary' | 'good' | 'down' | 'muted';

/** Border, background and text of a pill (`Trade/StatusPill`, footer price freshness). */
export const TRADE_TONE_CLASSES: Record<TradeTone, string> = {
  primary: 'border-primary/30 bg-primary/10 text-primary-ink',
  secondary: 'border-secondary/30 bg-secondary/10 text-secondary-ink',
  good: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/30 dark:bg-emerald-400/10 dark:text-emerald-300',
  down: 'border-red-500/30 bg-red-500/10 text-red-600 dark:border-red-400/30 dark:bg-red-400/10 dark:text-red-400',
  muted:
    'border-slate-300 bg-slate-100 text-slate-500 dark:border-white/15 dark:bg-white/5 dark:text-slate-400',
};

/** Rating left to the partner: 1 to 5 stars, 0 if the rating was skipped, `null` while not given. */
export type TradeRating = number | null;

/** Value of a card line in cents: trend price × quantity, 0 when the price is unknown. */
export const tradeCardValue = (card: TradeCard): number =>
  (card.price_guide?.trend ?? 0) * card.quantity;

export const tradeCardKey = (card: TradeCard): string =>
  `${card.set_code}-${card.collector_number}-${card.language_code}-${card.foil}`;

export const tradeCardsTotal = (cards: TradeCard[]): number =>
  cards.reduce((s, c) => s + tradeCardValue(c), 0);

/** What the screen compares between the two sides: the price, or the inclusion rate. */
export type TradeValueMode = 'eur' | 'inclusion_rate';

const UNKNOWN_RATE = '—';

/** Inclusion rate (0 to 100) with one decimal: "4,9 %", "< 0,1 %" for any known rate below 0.1
 * (zero included), "—" when it is unknown. */
export const formatInclusionRate = (rate?: number): string => {
  if (rate === undefined) return UNKNOWN_RATE;
  if (rate < 0.1) return '< 0,1 %';
  const value = rate.toLocaleString('fr-FR', {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  });
  return `${value} %`;
};

/** Average inclusion rate of the cards that have one: "moy. 13,2 %", "—" when none has. */
export const tradeInclusionAverageLabel = (cards: TradeCard[]): string => {
  const rates = cards.flatMap((c) => (c.inclusion_rate === undefined ? [] : [c.inclusion_rate]));
  if (!rates.length) return UNKNOWN_RATE;
  const average = rates.reduce((s, r) => s + r, 0) / rates.length;
  return `moy. ${formatInclusionRate(average)}`;
};

export interface TradeStatusPresentation {
  label: string;
  tone: TradeTone;
}

/** Single status to label and tone mapping of the web app: the pill and the dot indicator take
 * their color from it. */
export const TRADE_STATUS_META: Record<TradeStatusParam, TradeStatusPresentation> = {
  PENDING: { label: 'En négociation', tone: 'primary' },
  ONE_ACCEPTED: { label: '1 acceptation', tone: 'primary' },
  FULLY_ACCEPTED: { label: 'Verrouillé', tone: 'secondary' },
  COMPLETED: { label: 'Échange réalisé', tone: 'good' },
  CLOSED: { label: 'Clôturée', tone: 'muted' },
  ABANDONED: { label: 'Abandonnée', tone: 'down' },
};

export const isTradeEditable = (status: TradeStatusParam) =>
  status === 'PENDING' || status === 'ONE_ACCEPTED';

export const isTradeReserved = (status: TradeStatusParam) =>
  status === 'ONE_ACCEPTED' || status === 'FULLY_ACCEPTED';

export const isTradeAbandonable = (status: TradeStatusParam) =>
  isTradeEditable(status) || status === 'FULLY_ACCEPTED';

/** The five steps of the nominal path (ABANDONED excluded), the web app's only step list, same
 * texts as iOS `TradeSteps`; `useTradeView` positions them against the status. */
export const TRADE_STEPS: { status: TradeStatusParam; title: string; detail: string }[] = [
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

/** Below 3 EUR (300 cents), a Trade counts as balanced. */
const EVEN_THRESHOLD = 300;

/** The value gap is too small to call for a settlement. `diff` = total received minus total
 * given, in cents. */
export const isTradeBalanced = (diff: number): boolean => Math.abs(diff) < EVEN_THRESHOLD;

/** Settlement half of the accept button: "payer 21 €", "recevoir 4 €", `null` when balanced.
 * `diff` = total received minus total given, in cents. */
export const tradeSettlementLabel = (diff: number): string | null => {
  if (isTradeBalanced(diff)) return null;
  const amount = formatPrice(Math.abs(diff));
  return diff > 0 ? `payer ${amount}` : `recevoir ${amount}`;
};

/** "Accepter et payer 21 €": the amount is part of the commitment, so it goes on the button. */
export const tradeAcceptLabel = (diff: number) => {
  const settlement = tradeSettlementLabel(diff);
  return settlement ? `Accepter et ${settlement}` : "Accepter l'échange";
};

export type TradeConfirmationKind = 'accept' | 'abandon' | 'modify';

/** Texts of the confirmations before a locking or irreversible step, identical to iOS
 * `TradeConfirmation`. `diff` is only used for acceptance. */
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
