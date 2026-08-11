import { describe, it, expect } from 'vitest';
import type { TradeCard } from '~/bindings/TradeCard';
import {
  TRADE_STATUS_META,
  TRADE_LIFECYCLE,
  isTradeEditable,
  isTradeReserved,
  toTradeStatus,
  tradeCardValue,
  tradeCardsTotal,
  type TradeStatus,
} from './trade';

const card = (overrides: Partial<TradeCard> = {}): TradeCard => ({
  set_code: 'ECL',
  collector_number: '166',
  language_code: 'FR',
  foil: false,
  name: 'Sol Ring',
  quantity: 1,
  price_guide: { low: 100, avg: 150, trend: 200 },
  scryfall_id: 'abc',
  the_gatherer_id: null,
  ...overrides,
});

const ALL_STATUSES: TradeStatus[] = [
  'PENDING',
  'ONE_ACCEPTED',
  'FULLY_ACCEPTED',
  'COMPLETED',
  'CLOSED',
  'ABANDONED',
];

describe('isTradeEditable', () => {
  it.each([
    ['PENDING', true],
    ['ONE_ACCEPTED', true],
    ['FULLY_ACCEPTED', false],
    ['COMPLETED', false],
    ['CLOSED', false],
    ['ABANDONED', false],
  ] satisfies [TradeStatus, boolean][])('%s -> %s', (status, expected) => {
    expect(isTradeEditable(status)).toBe(expected);
  });
});

describe('isTradeReserved', () => {
  it.each([
    ['PENDING', false],
    ['ONE_ACCEPTED', true],
    ['FULLY_ACCEPTED', true],
    ['COMPLETED', false],
    ['CLOSED', false],
    ['ABANDONED', false],
  ] satisfies [TradeStatus, boolean][])('%s -> %s', (status, expected) => {
    expect(isTradeReserved(status)).toBe(expected);
  });
});

describe('TRADE_STATUS_META', () => {
  it('has a label and tone for every trade status', () => {
    for (const status of ALL_STATUSES) {
      expect(TRADE_STATUS_META[status]).toBeDefined();
      expect(TRADE_STATUS_META[status].label).toBeTruthy();
      expect(TRADE_STATUS_META[status].tone).toBeTruthy();
    }
  });
});

describe('TRADE_LIFECYCLE', () => {
  it('describes the nominal path in order, excluding ABANDONED', () => {
    expect(TRADE_LIFECYCLE.map((s) => s.status)).toEqual([
      'PENDING',
      'ONE_ACCEPTED',
      'FULLY_ACCEPTED',
      'COMPLETED',
      'CLOSED',
    ]);
  });

  it('gives every step a non-empty label', () => {
    for (const step of TRADE_LIFECYCLE) {
      expect(step.label).toBeTruthy();
    }
  });
});

describe('toTradeStatus', () => {
  it.each(ALL_STATUSES)('accepts %s as a valid status', (status) => {
    expect(toTradeStatus(status)).toBe(status);
  });

  it('falls back to PENDING for an unknown status', () => {
    expect(toTradeStatus('SOMETHING_UNEXPECTED')).toBe('PENDING');
  });
});

describe('tradeCardValue', () => {
  it('multiplies the trend price by the quantity', () => {
    expect(tradeCardValue(card({ quantity: 3, price_guide: { low: 0, avg: 0, trend: 200 } }))).toBe(
      600,
    );
  });

  it('is 0 when the price guide is unknown', () => {
    expect(tradeCardValue(card({ quantity: 5, price_guide: null }))).toBe(0);
  });
});

describe('tradeCardsTotal', () => {
  it('is 0 for an empty list', () => {
    expect(tradeCardsTotal([])).toBe(0);
  });

  it('sums the value of every card', () => {
    const cards = [
      card({ quantity: 1, price_guide: { low: 0, avg: 0, trend: 200 } }),
      card({ quantity: 2, price_guide: { low: 0, avg: 0, trend: 300 } }),
    ];
    expect(tradeCardsTotal(cards)).toBe(200 + 600);
  });
});
