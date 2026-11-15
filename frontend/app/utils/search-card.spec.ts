import { describe, it, expect } from 'vitest';
import type { SearchCard } from '~/bindings/SearchCard';
import { isFoilOnly, searchCardName, searchCardPrice, searchCardToDetail } from './search-card';

const guide = (trend: number | null) => ({ low: null, avg: null, trend });

const card = (overrides: Partial<SearchCard> = {}): SearchCard => ({
  set_code: 'LTR',
  set_name: 'The Lord of the Rings',
  collector_number: '246',
  rarity_code: 'R',
  scryfall_id: 'abc',
  name: 'Sol Ring',
  image_url: '/card-images/LTR_246_EN.webp?v=gatherer',
  image_back_url: null,
  owner_count: 2,
  owned_quantity: 0,
  offered_languages: [],
  price_guide: guide(500),
  foil_price_guide: guide(900),
  ...overrides,
});

describe('searchCardName', () => {
  it('shows the French name when there is one, the English one otherwise', () => {
    expect(searchCardName(card({ french_name: 'Anneau solaire' }))).toBe('Anneau solaire');
    expect(searchCardName(card())).toBe('Sol Ring');
  });
});

describe('the reference price', () => {
  it('is the normal trend when there is one', () => {
    expect(searchCardPrice(card())).toBe(500);
    expect(isFoilOnly(card())).toBe(false);
  });

  it('is the foil trend without a normal price', () => {
    const foilOnly = card({ price_guide: null });
    expect(searchCardPrice(foilOnly)).toBe(900);
    expect(isFoilOnly(foilOnly)).toBe(true);
  });

  it('is absent without any price', () => {
    const unpriced = card({ price_guide: null, foil_price_guide: null });
    expect(searchCardPrice(unpriced)).toBeUndefined();
    expect(isFoilOnly(unpriced)).toBe(false);
  });
});

describe('searchCardToDetail', () => {
  it('opens the English card with the normal price guide', () => {
    expect(searchCardToDetail(card({ french_name: 'Anneau solaire' }))).toMatchObject({
      set_code: 'LTR',
      collector_number: '246',
      language_code: 'EN',
      foil: false,
      name: 'Anneau solaire',
      collection_entry: null,
      reserved: false,
      price_guide: guide(500),
    });
  });

  it('opens the French card when it is offered', () => {
    expect(searchCardToDetail(card({ offered_languages: ['EN', 'FR'] })).language_code).toBe('FR');
    expect(searchCardToDetail(card({ offered_languages: ['FR'] })).language_code).toBe('FR');
  });

  it('opens the English card when French is not offered, or nothing is', () => {
    expect(searchCardToDetail(card({ offered_languages: ['EN', 'JA'] })).language_code).toBe('EN');
    expect(searchCardToDetail(card({ offered_languages: ['JA'] })).language_code).toBe('EN');
    expect(searchCardToDetail(card({ offered_languages: [] })).language_code).toBe('EN');
  });

  it('opens the foil card with the foil price guide for a foil only impression', () => {
    expect(searchCardToDetail(card({ price_guide: null }))).toMatchObject({
      foil: true,
      price_guide: guide(900),
    });
  });

  it('keeps the reservation of the searched player', () => {
    expect(searchCardToDetail(card({ reserved: true })).reserved).toBe(true);
  });
});
