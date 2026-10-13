import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';
import type { GraphRange } from './display-prefs';

const DAY_MS = 24 * 60 * 60 * 1000;

const dayLabelFormatter = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' });

const pctFormatter = new Intl.NumberFormat('fr-FR', {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});

/** Un point du graphe d'envelope : montants en euros, numéro de jour et libellé du jour. */
export type EnvelopePoint = { low: number; avg: number; trend: number; day: number; label: string };

/** Numéro de jour (jours depuis l'epoch) d'une date `YYYY-MM-DD`, insensible aux changements d'heure. */
const toDayNumber = (isoDate: string) => {
  const [year, month, day] = isoDate.split('-').map(Number);
  return Math.round(Date.UTC(year!, month! - 1, day) / DAY_MS);
};

export const formatDay = (dayNumber: number) => {
  const d = new Date(dayNumber * DAY_MS);
  return dayLabelFormatter.format(new Date(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()));
};

export const toEnvelopeData = (entries: PriceHistoryEntry[]) =>
  entries.map((e) => {
    const day = toDayNumber(e.date);
    return {
      low: e.low / 100,
      avg: e.avg / 100,
      trend: e.trend / 100,
      day,
      label: formatDay(day),
    };
  });

/** Graduations régulièrement espacées dans le temps, arrondies au jour et sans doublon. */
export const dayTicks = (firstDay: number, lastDay: number, count: number) => {
  const span = lastDay - firstDay;
  const steps = Math.min(count - 1, span);
  if (steps <= 0) return [firstDay];
  return Array.from({ length: steps + 1 }, (_, k) => firstDay + Math.round((span * k) / steps));
};

/** Indice du jour de `days` (triés) le plus proche de `day`. */
export const nearestIndex = (days: number[], day: number) => {
  let best = 0;
  for (let i = 1; i < days.length; i++) {
    if (Math.abs(days[i]! - day) < Math.abs(days[best]! - day)) best = i;
  }
  return best;
};

export const computeVariation = (entries: PriceHistoryEntry[]) => {
  if (entries.length < 2) return { pct: 0, deltaCents: 0, positive: true };
  const first = entries[0]!.trend;
  const last = entries[entries.length - 1]!.trend;
  const pct = first !== 0 ? ((last - first) / first) * 100 : 0;
  return { pct, deltaCents: last - first, positive: pct >= 0 };
};

/** Tout le modèle d'une tendance de prix, pour un Historique de prix en centimes trié par date. */
export const priceTrend = (entries: PriceHistoryEntry[]) => {
  const points: EnvelopePoint[] = toEnvelopeData(entries);
  const { pct, deltaCents, positive } = computeVariation(entries);
  return {
    points,
    hasEnoughHistory: points.length >= 2,
    lastTrendCents: entries.at(-1)?.trend ?? 0,
    variation: {
      positive,
      pctLabel: `${positive ? '+' : '−'}${pctFormatter.format(Math.abs(pct))} %`,
      deltaCents,
    },
  };
};

const toIsoDate = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

export const lastNDaysRange = (days: number) => {
  const end = new Date();
  const start = new Date(end);
  start.setDate(start.getDate() - (days - 1));
  return { start_date: toIsoDate(start), end_date: toIsoDate(end) };
};

/** Plage de dates `YYYY-MM-DD` d'une période, en heure locale, la fin étant aujourd'hui. */
export const periodDateRange = (period: GraphRange) => {
  if (period === '30 j') return lastNDaysRange(30);
  const end = new Date();
  const start = new Date(end);
  if (period === '3 m') start.setMonth(start.getMonth() - 3);
  else start.setFullYear(start.getFullYear() - (period === '1 an' ? 1 : 10));
  return { start_date: toIsoDate(start), end_date: toIsoDate(end) };
};
