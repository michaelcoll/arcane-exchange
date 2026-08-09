import { describe, it, expect } from 'vitest';
import {
  TRADE_STATUS_META,
  TRADE_LIFECYCLE,
  isTradeEditable,
  isTradeReserved,
  type TradeStatus,
} from './trade';

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
