import { describe, it, expect } from 'vitest';
import {
  canLoadPage,
  COLLECTION_MAX_OFFSET,
  SEARCH_MAX_OFFSET,
  TRADES_MAX_OFFSET,
} from './pagination';

describe('canLoadPage', () => {
  it('accepts a page whose offset is under the limit', () => {
    expect(canLoadPage(2, 20, 100)).toBe(true);
  });

  it('accepts a page whose offset lands exactly on the limit', () => {
    expect(canLoadPage(5, 20, 100)).toBe(true);
  });

  it('rejects a page whose offset exceeds the limit', () => {
    expect(canLoadPage(6, 20, 100)).toBe(false);
  });

  it('accepts page 0 regardless of the limit', () => {
    expect(canLoadPage(0, 20, 0)).toBe(true);
  });

  it('mirrors the backend offset limits for collection, search and trades', () => {
    expect(canLoadPage(COLLECTION_MAX_OFFSET / 20, 20, COLLECTION_MAX_OFFSET)).toBe(true);
    expect(canLoadPage(COLLECTION_MAX_OFFSET / 20 + 1, 20, COLLECTION_MAX_OFFSET)).toBe(false);
    expect(canLoadPage(SEARCH_MAX_OFFSET / 20, 20, SEARCH_MAX_OFFSET)).toBe(true);
    expect(canLoadPage(TRADES_MAX_OFFSET / 20, 20, TRADES_MAX_OFFSET)).toBe(true);
    expect(canLoadPage(TRADES_MAX_OFFSET / 20 + 1, 20, TRADES_MAX_OFFSET)).toBe(false);
  });
});
