<script
  setup
  lang="ts"
  generic="
    T extends { set_code: string; collector_number: string; language_code: string; foil: boolean }
  "
>
import type { RarityCode } from '~/bindings/RarityCode';
import type { SetInfo } from '~/bindings/SetInfo';
import type { CardList } from '~/composables/useCardList';
import type { CardSize } from '~/composables/useCardPageSize';

/**
 * The rendering shared by the pages that show a card list (collection, search): the filters (a
 * column on desktop, a bottom sheet on mobile), the states of the list, and the infinite scroll.
 * The page brings its toolbar, its card cell, its empty state and the text of its limit message.
 */
const props = defineProps<{
  /** The card list of the page. */
  list: CardList<T>;
  /** The sets offered by the filters. */
  setList: SetInfo[];
  /** The ends of the price slider, in euros. */
  priceMin: number;
  priceMax: number;
  size: CardSize;
  /** An overlay of the page is open (card detail, import): the body does not scroll behind it. */
  overlayOpen?: boolean;
}>();

defineSlots<{
  /** Above the filters, in their desktop column. */
  sidebar?(): unknown;
  /** The row above the list; `openFilters` opens the filter sheet (mobile). */
  toolbar(props: { openFilters: () => void }): unknown;
  empty(): unknown;
  cell(props: { card: T }): unknown;
  /** The text shown when the list stops at the offset limit of its endpoint. */
  limit(): unknown;
}>();

// The CardId and the finish: the foil and the normal copies of a card share their CardId.
const cardKey = (c: T) => `${c.set_code}-${c.collector_number}-${c.language_code}-${c.foil}`;

const sheet = ref(false);

const bodyScrollLocked = useScrollLock(document.body);
watchEffect(() => {
  bodyScrollLocked.value = sheet.value || !!props.overlayOpen;
});

// The filters name what they toggle by kind.
const filters = computed(() => ({
  active: { rar: [...props.list.criteria.rarity], sets: [...props.list.criteria.sets] },
  setList: props.setList,
  priceMin: props.priceMin,
  priceMax: props.priceMax,
  priceLo: props.list.priceLo,
  priceHi: props.list.priceHi,
}));
const toggle = (k: 'rar' | 'sets', v: string) =>
  k === 'rar' ? props.list.toggleRarity(v as RarityCode) : props.list.toggleSet(v);

// Infinite scroll: the list itself ignores the call while it loads or when nothing is left.
const sentinel = ref<HTMLElement | null>(null);
const { pause, resume } = useIntersectionObserver(
  sentinel,
  ([entry]) => {
    if (entry?.isIntersecting) props.list.loadMore();
  },
  { rootMargin: '300px' },
);
// The observer only reports a change of visibility: a page that leaves the sentinel in view (large
// screen, small page) would be the last one loaded. Observing afresh reports where the sentinel
// is now. Only once the list grew: a failed or an empty page is not asked again.
watch(
  () => props.list.cards.length,
  (count, previous) => {
    if (count <= previous) return;
    pause();
    resume();
  },
);
</script>

<template>
  <div class="flex items-start gap-6">
    <div class="sticky top-[86px] flex w-[210px] flex-none flex-col gap-3.5 max-md:hidden">
      <slot name="sidebar" />
      <aside
        class="rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <CollectionFilters v-bind="filters" @toggle="toggle" @price-change="list.setPriceRange" />
      </aside>
    </div>

    <div class="min-w-0 flex-1">
      <div class="mb-3.5 flex min-h-[22px] items-center justify-between gap-2.5">
        <slot name="toolbar" :open-filters="() => (sheet = true)" />
      </div>

      <div
        v-if="list.loading && list.cards.length === 0"
        class="flex items-center justify-center py-16 font-mono text-sm text-slate-400 dark:text-slate-500"
      >
        <Icon name="lucide:loader-circle" :size="18" class="mr-2.5 animate-spin" />
        Chargement…
      </div>

      <div
        v-else-if="list.cards.length === 0"
        class="flex flex-col items-center justify-center gap-4 py-20 text-slate-400 dark:text-slate-500"
      >
        <slot name="empty" />
      </div>

      <template v-else>
        <div
          :class="[
            'grid max-md:[grid-template-columns:repeat(auto-fill,minmax(150px,1fr))] max-md:gap-3.5',
            size === 'sm'
              ? '[grid-template-columns:repeat(auto-fill,minmax(130px,1fr))] gap-3'
              : '',
            size === 'md'
              ? '[grid-template-columns:repeat(auto-fill,minmax(185px,1fr))] gap-4'
              : '',
            size === 'lg'
              ? '[grid-template-columns:repeat(auto-fill,minmax(340px,1fr))] gap-6'
              : '',
          ]"
        >
          <template v-for="c in list.cards" :key="cardKey(c)">
            <slot name="cell" :card="c" />
          </template>
        </div>

        <div ref="sentinel" class="h-px" />
        <div
          v-if="list.loading"
          class="flex items-center justify-center py-8 font-mono text-sm text-slate-400 dark:text-slate-500"
        >
          <Icon name="lucide:loader-circle" :size="16" class="mr-2 animate-spin" />
          Chargement…
        </div>
        <div
          v-else-if="list.offsetLimitReached"
          class="flex items-center justify-center py-8 text-center font-mono text-sm text-slate-400 dark:text-slate-500"
        >
          <slot name="limit" />
        </div>
      </template>
    </div>
  </div>

  <div
    v-if="sheet"
    class="fixed inset-0 z-[80] animate-[fade_0.2s_ease] bg-black/60 backdrop-blur-sm"
    @click="sheet = false"
  >
    <div
      class="fixed right-0 bottom-0 left-0 z-[81] max-h-[84vh] animate-[slideup_0.3s_cubic-bezier(0.3,1,0.4,1)] overflow-auto rounded-t-3xl border-t border-slate-300 bg-white px-4 pt-5 pb-[calc(1.25rem+env(safe-area-inset-bottom))] shadow-2xl dark:border-white/15 dark:bg-zinc-900"
      @click.stop
    >
      <div class="mb-4 flex items-center justify-between">
        <h3 class="font-display text-base font-semibold tracking-tight">Filtres</h3>
        <button
          class="grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
          @click="sheet = false"
        >
          <Icon name="lucide:x" :size="16" />
        </button>
      </div>
      <CollectionFilters v-bind="filters" @toggle="toggle" @price-change="list.setPriceRange" />
      <button
        class="bg-primary hover:bg-primary-soft mt-4 inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0"
        @click="sheet = false"
      >
        Voir les résultats
      </button>
    </div>
  </div>
</template>
