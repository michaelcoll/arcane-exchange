import type { TradeViewSteps } from '~/composables/useTradeView';

/** Names of the steps as the player reads them, hard-coded here so as not to depend on the list
 * of the code. */
export const STEP_TITLES = [
  'Négociation',
  '1 acceptation',
  'Verrouillé',
  'Échange réalisé',
  'Clôturé',
];

const STATUSES = ['PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED', 'COMPLETED', 'CLOSED'] as const;

const stateAt = (i: number, currentIndex: number | null) => {
  if (currentIndex === null || i > currentIndex) return 'upcoming';
  return i < currentIndex ? 'done' : 'current';
};

/** The steps as the view of the Trade exposes them, the current step at `currentIndex` (`null`
 * for an abandoned Trade). */
export const stepsAt = (currentIndex: number | null): TradeViewSteps => ({
  currentIndex,
  list: STATUSES.map((status, i) => ({
    status,
    title: STEP_TITLES[i]!,
    detail: `Détail de l'étape ${i + 1}`,
    state: stateAt(i, currentIndex),
  })),
});
