import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import { effectScope, ref, type EffectScope, type MaybeRefOrGetter } from 'vue';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import { useSearchSets } from '~/composables/useSearchSets';

const { getSearchSetsMock } = vi.hoisted(() => ({
  getSearchSetsMock: vi.fn(),
}));

vi.mock('~/composables/useSearchService', () => ({
  useSearchService: () => ({ getSearchSets: getSearchSetsMock }),
}));

vi.mock('~/composables/useApi', () => ({
  useApi: () => ({ apiCall: vi.fn() }),
}));

const SETS = [{ code: 'LEA', name: 'Limited Edition Alpha' }];

// Like a page: the async data lives in a scope, disposed when the "page" goes away.
let pageScope: EffectScope;
const searchSets = (scope: MaybeRefOrGetter<SearchSetsParams | null>) =>
  pageScope.run(() => useSearchSets(scope))!;

describe('useSearchSets', () => {
  beforeEach(() => {
    pageScope = effectScope();
    getSearchSetsMock.mockReset();
    getSearchSetsMock.mockResolvedValue(SETS);
  });

  afterEach(() => {
    pageScope.stop();
    clearNuxtData('search-sets');
  });

  it('lists the sets of the scoped search', async () => {
    const { data } = await searchSets({ q: 'Sol Ring' });
    await flushPromises();

    expect(getSearchSetsMock).toHaveBeenCalledExactlyOnceWith({ q: 'Sol Ring' });
    expect(data.value).toEqual(SETS);
  });

  it('lists no set and calls nothing without a scope', async () => {
    const { data } = await searchSets(null);
    await flushPromises();

    expect(getSearchSetsMock).not.toHaveBeenCalled();
    expect(data.value).toEqual([]);
  });

  it('follows the scope when the search changes', async () => {
    const scope = ref<SearchSetsParams | null>({ q: 'Sol Ring' });

    await searchSets(scope);
    await flushPromises();
    scope.value = { player_username: 'urza' };
    await flushPromises();

    expect(getSearchSetsMock).toHaveBeenCalledTimes(2);
    expect(getSearchSetsMock).toHaveBeenLastCalledWith({ player_username: 'urza' });
  });

  it('does not refetch when the scope is rebuilt identical', async () => {
    const scope = ref<SearchSetsParams | null>({ player_username: 'urza' });

    await searchSets(scope);
    await flushPromises();
    scope.value = { player_username: 'urza' };
    await flushPromises();

    expect(getSearchSetsMock).toHaveBeenCalledTimes(1);
  });
});
