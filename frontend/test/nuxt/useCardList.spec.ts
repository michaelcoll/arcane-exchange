import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import { effectScope, ref, type EffectScope } from 'vue';
import {
  useCardList,
  type CardListOptions,
  type CardListPage,
  type CardListScope,
} from '~/composables/useCardList';
import {
  COLLECTION_CRITERIA,
  NAME_SEARCH_CRITERIA,
  PLAYER_SEARCH_CRITERIA,
  type CriteriaContext,
} from '~/utils/card-criteria-url';

type Scope = { q?: string };
type Options = CardListOptions<string, Scope>;

/** Opens the page at this URL, as a link or a reload would. */
const openAt = (query: Record<string, string> = {}) => useRouter().replace({ path: '/', query });
/** The URL once the navigation that follows a change has settled. */
const expectUrl = (query: Record<string, string>) =>
  vi.waitFor(() => expect(useRouter().currentRoute.value.query).toEqual(query));

/** `count` card names, numbered from `start`. */
const names = (start: number, count: number) =>
  Array.from({ length: count }, (_, i) => `card-${start + i}`);

/**
 * A fake page adapter over a collection of `total` cards: it records the parameters it receives
 * and answers the requested page.
 */
const fakeAdapter = (total: number) =>
  vi.fn<Options['fetchPage']>(async ({ page, page_size }) => ({
    items: names(page * page_size, Math.max(0, Math.min(page_size, total - page * page_size))),
    total,
  }));

/** A fake page adapter whose answers the test gives one by one, in the order it chooses. */
const deferredAdapter = () => {
  const answers: ((page: CardListPage<string>) => void)[] = [];
  const fetchPage = vi.fn<Options['fetchPage']>(
    () => new Promise<CardListPage<string>>((resolve) => answers.push(resolve)),
  );
  return {
    fetchPage,
    answer: (request: number, items: string[]) => answers[request]!({ items, total: items.length }),
  };
};

// Like a page: the list lives in a scope, disposed when the "page" goes away.
let pageScope: EffectScope;
const mountList = (options: Partial<Options> & Pick<Options, 'fetchPage'>) =>
  pageScope.run(() =>
    useCardList<string, Scope>({
      context: COLLECTION_CRITERIA,
      scope: { params: {}, query: {} },
      pageSize: 20,
      priceMin: 0,
      priceMax: 150,
      ...options,
    }),
  )!;

/** A list of `total` cards, opened and with its first page loaded. */
const openList = async (total: number, options: Partial<Options> = {}) => {
  const fetchPage = fakeAdapter(total);
  const list = mountList({ fetchPage, ...options });
  await flushPromises();
  return { list, fetchPage };
};

describe('useCardList', () => {
  beforeEach(async () => {
    pageScope = effectScope();
    await openAt();
  });

  afterEach(() => {
    pageScope.stop();
    vi.useRealTimers();
  });

  describe('opening the list', () => {
    it('loads once, with the criteria of the URL', async () => {
      await openAt({
        sort_by: 'trend',
        sort_dir: 'asc',
        rarity: 'M,R',
        sets: 'MH3,LTR',
        price_min: '500',
        price_max: '2000',
      });
      const fetchPage = fakeAdapter(3);

      const list = mountList({ fetchPage });
      expect(list.loading.value).toBe(true);
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledExactlyOnceWith({
        sort_by: 'trend',
        sort_dir: 'asc',
        rarity: ['M', 'R'],
        sets: 'MH3,LTR',
        price_min: 500,
        price_max: 2000,
        page: 0,
        page_size: 20,
      });
      expect(list.cards.value).toEqual(['card-0', 'card-1', 'card-2']);
      expect(list.total.value).toBe(3);
      expect(list.loading.value).toBe(false);
      expect(list.priceLo.value).toBe(5);
      expect(list.priceHi.value).toBe(20);
    });

    it('applies the valid criteria of a damaged URL and takes the rest out of it', async () => {
      await openAt({
        sort_by: 'name',
        sort_dir: 'desc',
        rarity: 'M,X',
        price_min: '-5',
        price_max: '300',
        unknown: '1',
      });

      const { fetchPage } = await openList(3);

      expect(fetchPage).toHaveBeenCalledExactlyOnceWith(
        expect.objectContaining({
          sort_by: 'added_at',
          sort_dir: 'desc',
          rarity: ['M'],
          price_min: undefined,
          price_max: 300,
        }),
      );
      await expectUrl({ rarity: 'M', price_max: '300' });
    });

    it('sends what the page scopes the list with, and keeps its parameters in the URL', async () => {
      await openAt({ q: 'sol', rarity: 'M' });

      const { fetchPage } = await openList(3, {
        scope: { params: { q: 'sol' }, query: { q: 'sol' } },
      });

      expect(fetchPage).toHaveBeenCalledExactlyOnceWith(
        expect.objectContaining({ q: 'sol', rarity: ['M'], page: 0 }),
      );
      await expectUrl({ q: 'sol', rarity: 'M' });
    });

    it('offers the sorts of its context', async () => {
      const { list } = await openList(3, { context: NAME_SEARCH_CRITERIA });

      expect(list.sorts.value.map((s) => s.value)).toEqual(['trend']);
    });
  });

  describe('loading the next page', () => {
    it('accumulates the pages until the last one', async () => {
      const { list, fetchPage } = await openList(45);
      expect(list.cards.value).toEqual(names(0, 20));
      expect(list.hasMore.value).toBe(true);

      list.loadMore();
      await flushPromises();
      expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({ page: 1 }));
      expect(list.cards.value).toEqual(names(0, 40));
      expect(list.hasMore.value).toBe(true);

      list.loadMore();
      await flushPromises();
      expect(list.cards.value).toEqual(names(0, 45));
      expect(list.total.value).toBe(45);
      expect(list.hasMore.value).toBe(false);
      expect(list.offsetLimitReached.value).toBe(false);
    });

    it('does nothing once every card is listed', async () => {
      const { list, fetchPage } = await openList(20);

      list.loadMore();
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(1);
      expect(list.cards.value).toEqual(names(0, 20));
    });

    it('does nothing while a page is loading', async () => {
      const { list, fetchPage } = await openList(100);

      list.loadMore();
      expect(list.loading.value).toBe(true);
      list.loadMore();
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(2);
      expect(list.cards.value).toEqual(names(0, 40));
    });

    it('stops before the page the endpoint would refuse, and says the limit is reached', async () => {
      // Offsets 0, 20 and 40 are within the limit; the page at offset 60 is not.
      const context: CriteriaContext = { ...COLLECTION_CRITERIA, maxOffset: 40 };
      const { list, fetchPage } = await openList(100, { context });

      list.loadMore();
      await flushPromises();
      expect(list.offsetLimitReached.value).toBe(false);
      list.loadMore();
      await flushPromises();

      expect(list.cards.value).toEqual(names(0, 60));
      expect(list.hasMore.value).toBe(false);
      expect(list.offsetLimitReached.value).toBe(true);

      list.loadMore();
      await flushPromises();
      expect(fetchPage).toHaveBeenCalledTimes(3);
    });

    it('leaves the list as it is when a page fails', async () => {
      const { list, fetchPage } = await openList(45);
      fetchPage.mockRejectedValueOnce(new Error('network down'));

      list.loadMore();
      await flushPromises();

      expect(list.cards.value).toEqual(names(0, 20));
      expect(list.loading.value).toBe(false);
      expect(list.hasMore.value).toBe(true);
    });
  });

  describe('changing what is listed', () => {
    const scope = ref<CardListScope<Scope>>({ params: {}, query: {} });
    const pageSize = ref(20);

    beforeEach(() => {
      scope.value = { params: {}, query: {} };
      pageSize.value = 20;
    });

    it.each<[string, (list: ReturnType<typeof mountList>) => void, object]>([
      ['a rarity', (list) => list.toggleRarity('M'), { rarity: ['M'] }],
      ['a set', (list) => list.toggleSet('MH3'), { sets: 'MH3' }],
      [
        'the sort',
        (list) => list.setSort({ sort_by: 'trend', sort_dir: 'asc' }),
        { sort_by: 'trend', sort_dir: 'asc' },
      ],
      [
        'the scope of the page',
        () => (scope.value = { params: { q: 'sol' }, query: { q: 'sol' } }),
        { q: 'sol' },
      ],
      ['the page size', () => (pageSize.value = 10), { page_size: 10 }],
    ])('starts over from the first page on a change of %s', async (_label, change, expected) => {
      const { list, fetchPage } = await openList(100, { scope, pageSize });
      list.loadMore();
      await flushPromises();
      expect(list.cards.value).toHaveLength(40);

      change(list);
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(3);
      expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({ page: 0, ...expected }));
      expect(list.cards.value).toEqual(names(0, pageSize.value));
    });

    it('lists no card of the previous criteria while the new first page loads', async () => {
      const { fetchPage, answer } = deferredAdapter();
      const list = mountList({ fetchPage });
      answer(0, ['a', 'b']);
      await flushPromises();
      expect(list.cards.value).toEqual(['a', 'b']);

      list.toggleRarity('M');
      await flushPromises();

      expect(list.cards.value).toEqual([]);
      expect(list.loading.value).toBe(true);
    });

    it('does not reload when the page rebuilds the same scope', async () => {
      scope.value = { params: { q: 'sol' }, query: { q: 'sol' } };
      const { fetchPage } = await openList(100, { scope });

      scope.value = { params: { q: 'sol' }, query: { q: 'sol' } };
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(1);
    });

    it('adds a rarity, then removes it when toggled again', async () => {
      const { list, fetchPage } = await openList(3);

      list.toggleRarity('M');
      list.toggleRarity('R');
      await flushPromises();
      expect(list.criteria.rarity).toEqual(['M', 'R']);
      expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({ rarity: ['M', 'R'] }));
      await expectUrl({ rarity: 'M,R' });

      list.toggleRarity('M');
      await flushPromises();
      expect(list.criteria.rarity).toEqual(['R']);
      expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({ rarity: ['R'] }));
      await expectUrl({ rarity: 'R' });
    });

    it('adds a set, then removes it when toggled again', async () => {
      const { list, fetchPage } = await openList(3);

      list.toggleSet('MH3');
      list.toggleSet('LTR');
      await flushPromises();
      expect(list.criteria.sets).toEqual(['MH3', 'LTR']);
      expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({ sets: 'MH3,LTR' }));
      await expectUrl({ sets: 'MH3,LTR' });

      list.toggleSet('MH3');
      list.toggleSet('LTR');
      await flushPromises();
      expect(list.criteria.sets).toEqual([]);
      expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({ sets: undefined }));
      await expectUrl({});
    });

    it('lists the cards of the latest request when an older one answers late', async () => {
      const { fetchPage, answer } = deferredAdapter();
      const list = mountList({ fetchPage });
      list.toggleRarity('M');
      await flushPromises();
      expect(fetchPage).toHaveBeenCalledTimes(2);

      answer(0, ['every rarity']);
      await flushPromises();
      expect(list.cards.value).toEqual([]);
      expect(list.loading.value).toBe(true);

      answer(1, ['mythic']);
      await flushPromises();
      expect(list.cards.value).toEqual(['mythic']);
      expect(list.loading.value).toBe(false);
    });

    it('drops the next page of a list whose criteria changed meanwhile', async () => {
      const { fetchPage, answer } = deferredAdapter();
      const list = mountList({ fetchPage });
      answer(0, ['a', 'b']);
      await flushPromises();
      // Two cards listed out of more: a next page exists.
      fetchPage.mockResolvedValueOnce({ items: ['c'], total: 5 });
      list.reload();
      await flushPromises();
      list.loadMore();
      list.toggleRarity('M');
      await flushPromises();

      answer(2, ['mythic']);
      answer(1, ['d']);
      await flushPromises();

      expect(list.cards.value).toEqual(['mythic']);
    });

    it('reloads from the first page on demand', async () => {
      const { list, fetchPage } = await openList(45);
      list.loadMore();
      await flushPromises();

      list.reload();
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(3);
      expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({ page: 0 }));
      expect(list.cards.value).toEqual(names(0, 20));
    });
  });

  describe('price range', () => {
    const openWithSlider = async (ends: Partial<Options> = {}) => {
      const opened = await openList(3, { priceMin: 0, priceMax: 150, ...ends });
      vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
      return opened;
    };

    it('filters in cents the range chosen in euros, once the slider rests for 300 ms', async () => {
      const { list, fetchPage } = await openWithSlider();

      list.setPriceRange(3, 40);
      list.setPriceRange(5, 20);
      await vi.advanceTimersByTimeAsync(299);
      expect(fetchPage).toHaveBeenCalledTimes(1);

      await vi.advanceTimersByTimeAsync(1);
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(2);
      expect(fetchPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ price_min: 500, price_max: 2000, page: 0 }),
      );
      expect(list.priceLo.value).toBe(5);
      expect(list.priceHi.value).toBe(20);
      await expectUrl({ price_min: '500', price_max: '2000' });
    });

    it('does not filter on a bound left at the end of the slider', async () => {
      const { list, fetchPage } = await openWithSlider();
      list.setPriceRange(5, 20);
      await vi.advanceTimersByTimeAsync(300);
      await flushPromises();

      list.setPriceRange(0, 150);
      await vi.advanceTimersByTimeAsync(300);
      await flushPromises();

      expect(fetchPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ price_min: undefined, price_max: undefined }),
      );
      expect(list.priceLo.value).toBeUndefined();
      expect(list.priceHi.value).toBeUndefined();
      await expectUrl({});
    });

    it('does not filter on the low bound left at the end of a slider that starts above 0', async () => {
      const { list, fetchPage } = await openWithSlider({ priceMin: 2 });

      list.setPriceRange(2, 20);
      await vi.advanceTimersByTimeAsync(300);
      await flushPromises();

      expect(fetchPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ price_min: undefined, price_max: 2000 }),
      );
      expect(list.priceLo.value).toBeUndefined();
      await expectUrl({ price_max: '2000' });
    });

    it('filters on a low bound moved off the end of a slider that starts above 0', async () => {
      const { list, fetchPage } = await openWithSlider({ priceMin: 2 });

      list.setPriceRange(3, 150);
      await vi.advanceTimersByTimeAsync(300);
      await flushPromises();

      expect(fetchPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ price_min: 300, price_max: undefined }),
      );
      await expectUrl({ price_min: '300' });
    });
  });

  describe('context', () => {
    it('opens a new context on its default sort, never sending a sort it does not offer', async () => {
      await openAt({ sort_dir: 'asc' });
      const context = ref(COLLECTION_CRITERIA);
      const scope = ref<CardListScope<Scope>>({ params: {}, query: {} });
      const { list, fetchPage } = await openList(3, { context, scope });
      expect(fetchPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ sort_by: 'added_at', sort_dir: 'asc' }),
      );

      // The page leaves a player's collection: context and scope change together.
      context.value = NAME_SEARCH_CRITERIA;
      scope.value = { params: { q: 'sol' }, query: { q: 'sol' } };
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(2);
      expect(fetchPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ q: 'sol', sort_by: 'trend', sort_dir: 'desc' }),
      );
      expect(list.criteria).toMatchObject({ sort_by: 'trend', sort_dir: 'desc' });
      await expectUrl({ q: 'sol' });
    });

    it('opens a new context on its default sort, even when it offers the current one', async () => {
      // A search by name, then a player is picked: their collection opens on date added.
      const context = ref(NAME_SEARCH_CRITERIA);
      const scope = ref<CardListScope<Scope>>({ params: {}, query: {} });
      const { list, fetchPage } = await openList(3, { context, scope });
      list.setSort({ sort_by: 'trend', sort_dir: 'asc' });
      await flushPromises();

      context.value = PLAYER_SEARCH_CRITERIA;
      scope.value = { params: { q: 'urza' }, query: { player: 'urza' } };
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(3);
      expect(fetchPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ q: 'urza', sort_by: 'added_at', sort_dir: 'desc' }),
      );
      expect(list.sorts.value.map((s) => s.value)).toEqual(['trend', 'added_at']);
      await expectUrl({ player: 'urza' });
    });

    it('keeps the sort of the URL when it opens in a context that offers it', async () => {
      await openAt({ player: 'urza', sort_by: 'trend', sort_dir: 'asc' });

      const { fetchPage } = await openList(3, {
        context: PLAYER_SEARCH_CRITERIA,
        scope: { params: { q: 'urza' }, query: { player: 'urza' } },
      });

      expect(fetchPage).toHaveBeenCalledExactlyOnceWith(
        expect.objectContaining({ q: 'urza', sort_by: 'trend', sort_dir: 'asc' }),
      );
      await expectUrl({ player: 'urza', sort_by: 'trend', sort_dir: 'asc' });
    });

    it('goes back to the default sort while no list is shown, without a request', async () => {
      // "Change player": the player's collection is left for the player picker.
      const context = ref(PLAYER_SEARCH_CRITERIA);
      const scope = ref<CardListScope<Scope>>({ params: {}, query: {} });
      const { list, fetchPage } = await openList(3, { context, scope });

      context.value = NAME_SEARCH_CRITERIA;
      scope.value = { params: null, query: { mode: 'player' } };
      await flushPromises();

      expect(fetchPage).toHaveBeenCalledTimes(1);
      expect(list.criteria).toMatchObject({ sort_by: 'trend', sort_dir: 'desc' });
      expect(list.cards.value).toEqual([]);
      await expectUrl({ mode: 'player' });
    });
  });

  describe('a page that shows no list', () => {
    it('requests nothing, and keeps only its own parameters in the URL', async () => {
      await openAt({ mode: 'decklist', rarity: 'M' });

      const { list, fetchPage } = await openList(3, {
        scope: { params: null, query: { mode: 'decklist' } },
      });
      list.loadMore();
      list.reload();
      await flushPromises();

      expect(fetchPage).not.toHaveBeenCalled();
      expect(list.loading.value).toBe(false);
      expect(list.cards.value).toEqual([]);
      await expectUrl({ mode: 'decklist' });
    });

    it('loads the list once the page shows one, and empties it when it no longer does', async () => {
      const scope = ref<CardListScope<Scope>>({ params: null, query: { mode: 'player' } });
      const { list, fetchPage } = await openList(3, { scope });

      scope.value = { params: { q: 'urza' }, query: { mode: 'player', player: 'urza' } };
      await flushPromises();
      expect(fetchPage).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ q: 'urza' }));
      expect(list.cards.value).toEqual(names(0, 3));

      scope.value = { params: null, query: { mode: 'player' } };
      await flushPromises();
      expect(fetchPage).toHaveBeenCalledTimes(1);
      expect(list.cards.value).toEqual([]);
      expect(list.total.value).toBe(0);
    });
  });

  describe('URL', () => {
    it('follows the criteria without adding a history entry', async () => {
      const { list } = await openList(3);
      const entries = window.history.length;

      list.toggleRarity('M');
      await flushPromises();
      list.setSort({ sort_by: 'trend', sort_dir: 'desc' });
      await flushPromises();

      await expectUrl({ rarity: 'M', sort_by: 'trend' });
      expect(window.history.length).toBe(entries);
    });

    it('gets the listed criteria back after a navigation to the same page without parameters', async () => {
      await openAt({ rarity: 'M' });
      const { list, fetchPage } = await openList(3);

      await useRouter().push({ path: '/' });
      await flushPromises();

      await expectUrl({ rarity: 'M' });
      expect(list.criteria.rarity).toEqual(['M']);
      expect(fetchPage).toHaveBeenCalledTimes(1);
    });

    it('leaves the URL of the next page alone while its own page is being left', async () => {
      const { list } = await openList(3);

      await useRouter().push({ path: '/sign-in' });
      list.toggleRarity('M');
      // Long enough for a navigation to settle, had the list started one.
      await new Promise((resolve) => setTimeout(resolve, 50));

      expect(useRouter().currentRoute.value.fullPath).toBe('/sign-in');
    });

    it('does not cancel a navigation to another page that is still pending', async () => {
      const { list } = await openList(3);
      const router = useRouter();
      // Holds the navigation where a middleware or a lazy page would.
      let release = () => {};
      const held = new Promise<void>((resolve) => (release = resolve));
      const removeGuard = router.beforeEach(() => held);

      const navigation = router.push({ path: '/sign-in' });
      await flushPromises();
      list.toggleRarity('M');
      await flushPromises();
      release();
      await navigation;
      removeGuard();

      expect(router.currentRoute.value.fullPath).toBe('/sign-in');
    });
  });
});
