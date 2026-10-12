import { useRoute } from 'nuxt/app';
import type { Reactive } from 'vue';
import type { RarityCode } from '~/bindings/RarityCode';
import type { SortBy } from '~/bindings/SortBy';
import type { SortDir } from '~/bindings/SortDir';
import type { CriteriaContext } from '~/utils/card-criteria-url';

/** One page of a card list, as its endpoint answers it. */
export interface CardListPage<T> {
  items: T[];
  total: number;
}

/** The criteria and the pagination of a request, in the shape of the API parameters. */
export interface CardListParams {
  sort_by: SortBy;
  sort_dir: SortDir;
  rarity: RarityCode[];
  sets?: string;
  price_min?: number;
  price_max?: number;
  page: number;
  page_size: number;
}

/**
 * What a page adds to its card list, opaque to the list: the request parameters next to the
 * criteria (text, player) and the URL parameters next to them. `params: null` means the page
 * shows no list: nothing is requested, and the criteria stay out of the URL.
 */
export interface CardListScope<S> {
  params: S | null;
  query: Record<string, string>;
}

export interface CardListOptions<T, S> {
  /** The sorts offered, the default one and how deep the endpoint paginates. */
  context: MaybeRefOrGetter<CriteriaContext>;
  /** The page adapter: fetches one page of cards. */
  fetchPage: (params: CardListParams & S) => Promise<CardListPage<T>>;
  scope: MaybeRefOrGetter<CardListScope<S>>;
  pageSize: MaybeRefOrGetter<number>;
  /** The ends of the price slider, in euros: a bound left at an end is no filter. */
  priceMin: MaybeRefOrGetter<number>;
  priceMax: MaybeRefOrGetter<number>;
}

const toggled = <V>(values: V[], value: V) =>
  values.includes(value) ? values.filter((v) => v !== value) : [...values, value];

/**
 * The state of a card list (collection, search): its criteria read from and kept in the URL, the
 * cards accumulated page after page, and the loading state. The page brings its context, its page
 * adapter and its scope; any change of criteria, scope, context or page size starts over from the
 * first page, and a change of context from its default sort too.
 */
export const useCardList = <T, S extends object>(options: CardListOptions<T, S>) => {
  const route = useRoute();
  const context = computed(() => toValue(options.context));
  const scope = computed(() => toValue(options.scope));

  // Read from the URL before the first request: an invalid value falls back to its default here,
  // and leaves the URL at the first sync.
  const criteria = reactive(parseCardCriteria(route.query, context.value));

  // A new context opens on its default sort, in the same update as the context: a sort it does
  // not offer never reaches the adapter.
  watch(context, ({ defaultSort }) => Object.assign(criteria, defaultSort), { flush: 'sync' });

  /** The request of the first page, `null` when the page shows no list. */
  const firstPage = computed((): (CardListParams & S) | null => {
    const { params } = scope.value;
    if (!params) return null;
    return {
      ...params,
      sort_by: criteria.sort_by,
      sort_dir: criteria.sort_dir,
      rarity: criteria.rarity,
      sets: criteria.sets.length ? criteria.sets.join(',') : undefined,
      price_min: criteria.price_min,
      price_max: criteria.price_max,
      page: 0,
      page_size: toValue(options.pageSize),
    };
  });

  const cards = shallowRef<T[]>([]);
  const total = ref(0);
  const loading = ref(false);
  const loadedPages = ref(0);
  // The latest request: the answer of an older one is dropped.
  let latest = 0;

  const load = async (params: CardListParams & S) => {
    const request = ++latest;
    loading.value = true;
    try {
      const page = await options.fetchPage(params);
      if (request !== latest) return;
      cards.value = [...cards.value, ...page.items];
      total.value = page.total;
      loadedPages.value = params.page + 1;
    } catch {
      // A failed page leaves the list as it is.
    } finally {
      if (request === latest) loading.value = false;
    }
  };

  const reload = () => {
    latest++;
    cards.value = [];
    total.value = 0;
    loadedPages.value = 0;
    loading.value = false;
    if (firstPage.value) load(firstPage.value);
  };

  // Compared by value: a scope rebuilt identical does not reload.
  watch(() => JSON.stringify(firstPage.value), reload);
  reload();

  const moreCardsExist = computed(() => cards.value.length < total.value);
  const withinOffsetLimit = computed(() =>
    canLoadPage(loadedPages.value, toValue(options.pageSize), context.value.maxOffset),
  );
  const hasMore = computed(() => moreCardsExist.value && withinOffsetLimit.value);
  // Distinct from "no more cards": there are more, but paging further would hit the offset limit
  // of the endpoint. Surfaced so that the stop does not read as an empty or broken list.
  const offsetLimitReached = computed(() => moreCardsExist.value && !withinOffsetLimit.value);

  const loadMore = () => {
    if (loading.value || !hasMore.value || !firstPage.value) return;
    load({ ...firstPage.value, page: loadedPages.value });
  };

  const { sync } = useQuerySync(() => ({
    ...scope.value.query,
    ...(scope.value.params ? toCardCriteriaQuery(criteria, context.value) : {}),
  }));
  tryOnMounted(sync);

  return {
    cards: readonly(cards),
    total: readonly(total),
    loading: readonly(loading),
    hasMore,
    offsetLimitReached,
    criteria: readonly(criteria),
    sorts: computed(() => context.value.sorts),
    /** The chosen price bounds in euros, for the slider. */
    priceLo: computed(() => centsToEuros(criteria.price_min)),
    priceHi: computed(() => centsToEuros(criteria.price_max)),
    toggleRarity: (rarity: RarityCode) => {
      criteria.rarity = toggled(criteria.rarity, rarity);
    },
    toggleSet: (code: string) => {
      criteria.sets = toggled(criteria.sets, code);
    },
    setSort: (sort: { sort_by: SortBy; sort_dir: SortDir }) => {
      criteria.sort_by = sort.sort_by;
      criteria.sort_dir = sort.sort_dir;
    },
    /** The price range chosen on the slider, in euros. */
    setPriceRange: useDebounceFn((lo: number, hi: number) => {
      criteria.price_min = lo > toValue(options.priceMin) ? lo * 100 : undefined;
      criteria.price_max = hi < toValue(options.priceMax) ? hi * 100 : undefined;
    }, 300),
    loadMore,
    reload,
  };
};

/**
 * A card list as its page holds it, unwrapped with `reactive`. The cards are spelled out so that
 * `T` is inferred from the list a page passes on.
 */
export type CardList<T> = Omit<
  Reactive<ReturnType<typeof useCardList<unknown, object>>>,
  'cards'
> & {
  cards: readonly T[];
};
