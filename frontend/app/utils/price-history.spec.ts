import { describe, it, expect, vi, afterEach } from 'vitest';
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';
import {
  toEnvelopeData,
  computeVariation,
  lastNDaysRange,
  dayTicks,
  nearestIndex,
  formatDay,
} from './price-history';

const entry = (overrides: Partial<PriceHistoryEntry> = {}): PriceHistoryEntry => ({
  date: '2026-01-15',
  low: 1000,
  avg: 1200,
  trend: 1500,
  ...overrides,
});

const dayLabelFormatter = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' });
const labelFor = (year: number, month: number, day: number) =>
  dayLabelFormatter.format(new Date(year, month - 1, day));

describe('toEnvelopeData', () => {
  it('converts cent amounts to euros', () => {
    const [point] = toEnvelopeData([entry({ low: 1000, avg: 1200, trend: 1500 })]);
    expect(point).toMatchObject({ low: 10, avg: 12, trend: 15 });
  });

  it('formats the date into a localized day/month label', () => {
    const [point] = toEnvelopeData([entry({ date: '2026-03-05' })]);
    expect(point!.label).toBe(labelFor(2026, 3, 5));
  });

  it('maps every entry in order', () => {
    const data = toEnvelopeData([
      entry({ date: '2026-01-01', trend: 100 }),
      entry({ date: '2026-01-02', trend: 200 }),
    ]);
    expect(data.map((d) => d.trend)).toEqual([1, 2]);
  });

  it('returns an empty array for no entries', () => {
    expect(toEnvelopeData([])).toEqual([]);
  });

  it('numbers the days so that gaps between entries keep their real length', () => {
    const data = toEnvelopeData([
      entry({ date: '2026-09-12' }),
      entry({ date: '2026-09-25' }),
      entry({ date: '2026-09-27' }),
    ]);
    const [a, b, c] = data.map((d) => d.day);
    expect(b! - a!).toBe(13);
    expect(c! - b!).toBe(2);
  });

  it('counts a DST change as a single day', () => {
    const [a, b] = toEnvelopeData([entry({ date: '2026-10-24' }), entry({ date: '2026-10-26' })]);
    expect(b!.day - a!.day).toBe(2);
  });
});

describe('formatDay', () => {
  it('labels a day number like its entry', () => {
    const [point] = toEnvelopeData([entry({ date: '2026-03-05' })]);
    expect(formatDay(point!.day)).toBe(labelFor(2026, 3, 5));
  });
});

describe('dayTicks', () => {
  it('spreads ticks evenly in time from the first to the last day', () => {
    expect(dayTicks(100, 130, 6)).toEqual([100, 106, 112, 118, 124, 130]);
  });

  it('rounds ticks to whole days', () => {
    expect(dayTicks(0, 7, 3)).toEqual([0, 4, 7]);
  });

  it('never repeats a day when the span is shorter than the tick count', () => {
    expect(dayTicks(10, 12, 6)).toEqual([10, 11, 12]);
  });

  it('returns a single tick for a single day', () => {
    expect(dayTicks(10, 10, 6)).toEqual([10]);
  });
});

describe('nearestIndex', () => {
  const days = [0, 3, 16, 18];

  it('picks the entry closest in time, not by position', () => {
    expect(nearestIndex(days, 8)).toBe(1);
    expect(nearestIndex(days, 12)).toBe(2);
  });

  it('clamps outside the range', () => {
    expect(nearestIndex(days, -5)).toBe(0);
    expect(nearestIndex(days, 40)).toBe(3);
  });
});

describe('computeVariation', () => {
  it('reports a flat, positive-by-default variation for fewer than 2 entries', () => {
    expect(computeVariation([])).toEqual({ pct: 0, deltaCents: 0, positive: true });
    expect(computeVariation([entry()])).toEqual({ pct: 0, deltaCents: 0, positive: true });
  });

  it('computes the percentage and cent delta between the first and last entry', () => {
    const result = computeVariation([
      entry({ trend: 1000 }),
      entry({ trend: 1100 }),
      entry({ trend: 1200 }),
    ]);
    expect(result).toEqual({ pct: 20, deltaCents: 200, positive: true });
  });

  it('flags a decrease as negative', () => {
    const result = computeVariation([entry({ trend: 1000 }), entry({ trend: 800 })]);
    expect(result).toEqual({ pct: -20, deltaCents: -200, positive: false });
  });

  it('treats a zero variation as positive', () => {
    const result = computeVariation([entry({ trend: 1000 }), entry({ trend: 1000 })]);
    expect(result).toEqual({ pct: 0, deltaCents: 0, positive: true });
  });

  it('avoids dividing by zero when the first entry has no value', () => {
    const result = computeVariation([entry({ trend: 0 }), entry({ trend: 500 })]);
    expect(result).toEqual({ pct: 0, deltaCents: 500, positive: true });
  });
});

describe('lastNDaysRange', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns a start/end range spanning N days, inclusive of today', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 2, 15));
    expect(lastNDaysRange(30)).toEqual({ start_date: '2026-02-14', end_date: '2026-03-15' });
  });

  it('zero-pads single-digit months and days', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 0, 5));
    expect(lastNDaysRange(3)).toEqual({ start_date: '2026-01-03', end_date: '2026-01-05' });
  });

  it('returns today for both bounds when asking for a single day', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 1));
    expect(lastNDaysRange(1)).toEqual({ start_date: '2026-06-01', end_date: '2026-06-01' });
  });
});
