import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import { effectScope, ref, type EffectScope, type MaybeRefOrGetter } from 'vue';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import { useSearchService } from '~/composables/useSearchService';

const { apiCallMock } = vi.hoisted(() => ({
  apiCallMock: vi.fn(),
}));

vi.mock('~/composables/useApi', () => ({
  useApi: () => ({ apiCall: apiCallMock }),
}));

const SETS = [{ code: 'LEA', name: 'Limited Edition Alpha' }];

// Like a page: the async data lives in a scope, disposed when the "page" goes away.
let pageScope: EffectScope;
const getSearchSets = (scope: MaybeRefOrGetter<SearchSetsParams | null>) =>
  pageScope.run(() => useSearchService().getSearchSets(scope))!;

describe('useSearchService.getSearchSets', () => {
  beforeEach(() => {
    pageScope = effectScope();
    apiCallMock.mockReset();
    apiCallMock.mockResolvedValue(SETS);
  });

  afterEach(() => {
    pageScope.stop();
    clearNuxtData('search-sets');
  });

  it('lists the sets of the scoped search, from the search facet', async () => {
    const { data } = await getSearchSets({ q: 'Sol Ring' });
    await flushPromises();

    expect(apiCallMock).toHaveBeenCalledExactlyOnceWith('/search/card/sets', {
      query: { q: 'Sol Ring' },
    });
    expect(data.value).toEqual(SETS);
  });

  it('lists no set and calls nothing without a scope', async () => {
    const { data } = await getSearchSets(null);
    await flushPromises();

    expect(apiCallMock).not.toHaveBeenCalled();
    expect(data.value).toEqual([]);
  });

  it('follows the scope when the search changes', async () => {
    const scope = ref<SearchSetsParams | null>({ q: 'Sol Ring' });

    await getSearchSets(scope);
    await flushPromises();
    scope.value = { player_username: 'urza' };
    await flushPromises();

    expect(apiCallMock).toHaveBeenCalledTimes(2);
    expect(apiCallMock).toHaveBeenLastCalledWith('/search/card/sets', {
      query: { player_username: 'urza' },
    });
  });

  it('does not refetch when the scope is rebuilt identical', async () => {
    const scope = ref<SearchSetsParams | null>({ player_username: 'urza' });

    await getSearchSets(scope);
    await flushPromises();
    scope.value = { player_username: 'urza' };
    await flushPromises();

    expect(apiCallMock).toHaveBeenCalledTimes(1);
  });
});
