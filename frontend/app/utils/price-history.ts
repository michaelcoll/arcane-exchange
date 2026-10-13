import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';
import type { GraphRange } from './display-prefs';

const DAY_MS = 24 * 60 * 60 * 1000;

const dayLabelFormatter = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' });

const pctFormatter = new Intl.NumberFormat('fr-FR', {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});

/** A point of the envelope graph: amounts in euros, day number and day label. */
export type EnvelopePoint = { low: number; avg: number; trend: number; day: number; label: string };

/** Day number (days since the epoch) of a `YYYY-MM-DD` date, unaffected by clock changes. */
const toDayNumber = (isoDate: string) => {
  const [year, month, day] = isoDate.split('-').map(Number);
  return Math.round(Date.UTC(year!, month! - 1, day) / DAY_MS);
};

export const formatDay = (dayNumber: number) => {
  const d = new Date(dayNumber * DAY_MS);
  return dayLabelFormatter.format(new Date(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()));
};

const toEnvelopeData = (entries: PriceHistoryEntry[]): EnvelopePoint[] =>
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

/** Ticks evenly spaced in time, rounded to the day and without duplicates. */
export const dayTicks = (firstDay: number, lastDay: number, count: number) => {
  const span = lastDay - firstDay;
  const steps = Math.min(count - 1, span);
  if (steps <= 0) return [firstDay];
  return Array.from({ length: steps + 1 }, (_, k) => firstDay + Math.round((span * k) / steps));
};

/** Index of the day of `days` (sorted) closest to `day`. */
export const nearestIndex = (days: number[], day: number) => {
  let best = 0;
  for (let i = 1; i < days.length; i++) {
    if (Math.abs(days[i]! - day) < Math.abs(days[best]! - day)) best = i;
  }
  return best;
};

const computeVariation = (entries: PriceHistoryEntry[]) => {
  if (entries.length < 2) return { pct: 0, deltaCents: 0, positive: true };
  const first = entries[0]!.trend;
  const last = entries[entries.length - 1]!.trend;
  const pct = first !== 0 ? ((last - first) / first) * 100 : 0;
  return { pct, deltaCents: last - first, positive: pct >= 0 };
};

/**
 * The whole model of a price trend, for a series in cents sorted by date: the price history of a
 * card as well as the value history of a collection, which share the same entry shape.
 */
export const priceTrend = (entries: PriceHistoryEntry[]) => {
  const points = toEnvelopeData(entries);
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

/** `YYYY-MM-DD` date range of a period, in local time, ending today. */
export const periodDateRange = (period: GraphRange) => {
  const end = new Date();
  const start = new Date(end);
  // '30 j' includes today: the start is 29 days earlier.
  if (period === '30 j') start.setDate(start.getDate() - 29);
  else if (period === '3 m') start.setMonth(start.getMonth() - 3);
  else if (period === '1 an') start.setFullYear(start.getFullYear() - 1);
  // 'Max' has no real lower bound yet: ten years back.
  else start.setFullYear(start.getFullYear() - 10);
  return { start_date: toIsoDate(start), end_date: toIsoDate(end) };
};
