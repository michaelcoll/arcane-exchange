import { describe, it, expect } from 'vitest';
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeStatusParam } from '~/bindings/TradeStatusParam';
import {
  TRADE_STATUS_META,
  TRADE_STEPS,
  isTradeEditable,
  isTradeAbandonable,
  isTradeReserved,
  isTradeBalanced,
  toTradeStatus,
  tradeAcceptLabel,
  tradeCardKey,
  tradeCardValue,
  tradeCardsTotal,
  tradeConfirmation,
  tradeSettlementLabel,
} from './trade';
import formatPrice from './format-price';

const card = (overrides: Partial<TradeCard> = {}): TradeCard => ({
  set_code: 'ECL',
  collector_number: '166',
  language_code: 'FR',
  foil: false,
  name: 'Sol Ring',
  quantity: 1,
  price_guide: { low: 100, avg: 150, trend: 200 },
  scryfall_id: 'abc',
  image_url: null,
  image_back_url: null,
  ...overrides,
});

const ALL_STATUSES: TradeStatusParam[] = [
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
  ] satisfies [TradeStatusParam, boolean][])('%s -> %s', (status, expected) => {
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
  ] satisfies [TradeStatusParam, boolean][])('%s -> %s', (status, expected) => {
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

describe('isTradeAbandonable', () => {
  it.each([
    ['PENDING', true],
    ['ONE_ACCEPTED', true],
    ['FULLY_ACCEPTED', true],
    ['COMPLETED', false],
    ['CLOSED', false],
    ['ABANDONED', false],
  ] satisfies [TradeStatusParam, boolean][])('%s -> %s', (status, expected) => {
    expect(isTradeAbandonable(status)).toBe(expected);
  });
});

describe('isTradeBalanced', () => {
  it('holds while the cash delta stays under 3 €, whoever owes it', () => {
    expect(isTradeBalanced(0)).toBe(true);
    expect(isTradeBalanced(299)).toBe(true);
    expect(isTradeBalanced(-299)).toBe(true);
  });

  it('stops at 3 €', () => {
    expect(isTradeBalanced(300)).toBe(false);
    expect(isTradeBalanced(-300)).toBe(false);
  });

  it.each([0, 299, -299, 300, -300, 2100])(
    'announces a settlement exactly when the trade is not balanced (%i)',
    (diff) => {
      expect(tradeSettlementLabel(diff) === null).toBe(isTradeBalanced(diff));
    },
  );
});

describe('tradeCardKey', () => {
  it('tells apart two finishes of the same CardId', () => {
    expect(tradeCardKey(card({ foil: false }))).not.toBe(tradeCardKey(card({ foil: true })));
  });

  it('tells apart two CardIds', () => {
    const keys = [
      card(),
      card({ set_code: 'MH3' }),
      card({ collector_number: '167' }),
      card({ language_code: 'EN' }),
    ].map(tradeCardKey);
    expect(new Set(keys).size).toBe(4);
  });

  it('ignores what is not the CardId or the finish', () => {
    expect(tradeCardKey(card({ quantity: 4, name: 'Autre' }))).toBe(tradeCardKey(card()));
  });
});

describe('tradeSettlementLabel', () => {
  it('is null when both sides are within 3 € of each other', () => {
    expect(tradeSettlementLabel(0)).toBeNull();
    expect(tradeSettlementLabel(299)).toBeNull();
    expect(tradeSettlementLabel(-299)).toBeNull();
  });

  it('asks me to pay when I receive more than I give', () => {
    expect(tradeSettlementLabel(2100)).toBe(`payer ${formatPrice(2100)}`);
  });

  it('tells me I receive when I give more than I get', () => {
    expect(tradeSettlementLabel(-400)).toBe(`recevoir ${formatPrice(400)}`);
  });
});

describe('tradeAcceptLabel', () => {
  it('puts the settlement amount on the button', () => {
    expect(tradeAcceptLabel(2100)).toBe(`Accepter et payer ${formatPrice(2100)}`);
    expect(tradeAcceptLabel(-400)).toBe(`Accepter et recevoir ${formatPrice(400)}`);
  });

  it('falls back to a plain label when the values are even', () => {
    expect(tradeAcceptLabel(100)).toBe("Accepter l'échange");
  });
});

describe('tradeConfirmation', () => {
  it('spells out the settlement before accepting', () => {
    const c = tradeConfirmation('accept', 2100);
    expect(c.title).toBe('Accepter cet échange ?');
    expect(c.body).toMatch(
      new RegExp(`^Tu t'engages à payer ${formatPrice(2100)} en main propre\\. `),
    );
    expect(c.confirmLabel).toBe('Accepter');
    expect(c.tone).toBe('primary');
  });

  it('says no settlement is due when the values are even', () => {
    expect(tradeConfirmation('accept', 0).body).toMatch(
      /^Les valeurs sont équivalentes, aucun règlement\. Les cartes des deux côtés seront réservées\./,
    );
  });

  it('warns before abandoning', () => {
    expect(tradeConfirmation('abandon', 0)).toEqual({
      title: "Abandonner l'échange ?",
      body: "L'échange sera définitivement abandonné et les cartes réservées libérées. Action irréversible.",
      confirmLabel: 'Abandonner',
      tone: 'down',
    });
  });

  it('warns before modifying an accepted trade', () => {
    expect(tradeConfirmation('modify', 0)).toEqual({
      title: "Modifier l'échange ?",
      body: 'Une partie a déjà accepté. Modifier libère les cartes réservées, annule les acceptations et relance la négociation.',
      confirmLabel: 'Modifier quand même',
      tone: 'down',
    });
  });
});

describe('TRADE_STEPS', () => {
  it('explains the nominal path in order, excluding ABANDONED', () => {
    expect(TRADE_STEPS.map((s) => s.status)).toEqual([
      'PENDING',
      'ONE_ACCEPTED',
      'FULLY_ACCEPTED',
      'COMPLETED',
      'CLOSED',
    ]);
    for (const step of TRADE_STEPS) {
      expect(step.title).toBeTruthy();
      expect(step.detail).toBeTruthy();
    }
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
