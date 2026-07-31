<script setup lang="ts">
import type { CollectionCard } from '~/bindings/CollectionCard';
import type { RarityCode } from '~/bindings/RarityCode';

const { getCollectionStats } = useCollectionService();
const { getSearch } = useSearchService();

const mode = ref<'name' | 'decklist'>('name');

const modeOptions = [
  { value: 'name', label: 'Par nom', tone: 'cyan' },
  { value: 'decklist', label: 'Par decklist', tone: 'cyan' },
];

/* ---------- MODE: PAR NOM ---------- */
const q = ref('');
const submittedQ = ref('');

const size = ref<'sm' | 'md' | 'lg'>('md');
const isDesktop = useMediaQuery('(min-width: 768px)');
const pageSize = useCardPageSize(size, isDesktop);

const params = ref({
  sort_by: 'trend' as const,
  sort_dir: 'desc' as const,
  page: 0,
  page_size: pageSize.value,
  q: '',
  rarity: [] as RarityCode[],
  sets: undefined as string | undefined,
  price_min: undefined as number | undefined,
  price_max: undefined as number | undefined,
});

const { data: collectionData, pending, refresh } = await getSearch(params);
const { data: statsData } = await getCollectionStats();

const allCards = ref<CollectionCard[]>([]);

watch(
  collectionData,
  (data) => {
    if (!data) return;
    if (params.value.page === 0) {
      allCards.value = [...data.items];
    } else {
      allCards.value.push(...data.items);
    }
  },
  { immediate: true },
);

const resetAndRefresh = () => {
  allCards.value = [];
  params.value.page = 0;
  refresh();
};

const submitSearch = () => {
  submittedQ.value = q.value;
  params.value.q = q.value;
  resetAndRefresh();
};

watch(pageSize, (v) => {
  params.value.page_size = v;
  resetAndRefresh();
});

watch(
  () => params.value.page,
  (page) => {
    if (page > 0) refresh();
  },
);

const hasMore = computed(() =>
  collectionData.value ? allCards.value.length < collectionData.value.total : false,
);

const sentinel = ref<HTMLElement | null>(null);
let io: IntersectionObserver | null = null;

onMounted(() => {
  io = new IntersectionObserver(
    ([entry]) => {
      if (entry?.isIntersecting && hasMore.value && !pending.value) {
        params.value.page += 1;
      }
    },
    { rootMargin: '300px' },
  );
  onUnmounted(() => io?.disconnect());
});

watch(sentinel, (el, oldEl) => {
  if (oldEl) io?.unobserve(oldEl);
  if (el) io?.observe(el);
});

const sheet = ref(false);
const active = ref({ rar: [] as RarityCode[], sets: [] as string[] });
const detail = ref<CollectionCard | null>(null);

const bodyScrollLocked = useScrollLock(document.body);
watch([detail, sheet], ([d, s]) => {
  bodyScrollLocked.value = !!d || s;
});

const toggle = (k: 'rar' | 'sets', v: string) => {
  if (k === 'rar') {
    const arr = active.value.rar;
    active.value.rar = arr.includes(v as RarityCode)
      ? arr.filter((x) => x !== v)
      : [...arr, v as RarityCode];
  } else {
    const arr = active.value.sets;
    active.value.sets = arr.includes(v) ? arr.filter((x) => x !== v) : [...arr, v];
  }
};

watch(
  () => [active.value.rar, active.value.sets],
  () => {
    params.value.rarity = active.value.rar;
    params.value.sets = active.value.sets.length ? active.value.sets.join(',') : undefined;
    resetAndRefresh();
  },
  { deep: true },
);

const setList = computed(() => statsData.value?.sets ?? []);

const priceMin = computed(() =>
  statsData.value?.price_trend_min != null ? Math.floor(statsData.value.price_trend_min / 100) : 0,
);
const priceMax = computed(() =>
  statsData.value?.price_trend_max != null ? Math.ceil(statsData.value.price_trend_max / 100) : 150,
);

const onPriceChange = useDebounceFn((lo: number, hi: number) => {
  params.value.price_min = lo > 0 ? lo * 100 : undefined;
  params.value.price_max = hi < priceMax.value ? hi * 100 : undefined;
  resetAndRefresh();
}, 300);

const sizeOptions = [
  { value: 'sm', label: '', icon: 'lucide:grid-3x3', title: 'Petites cartes', tone: 'cyan' },
  { value: 'md', label: '', icon: 'lucide:grid-2x2', title: 'Cartes moyennes', tone: 'cyan' },
  { value: 'lg', label: '', icon: 'lucide:square', title: 'Grandes cartes', tone: 'cyan' },
];

/* ---------- MODE: PAR DECKLIST ---------- */
const coverers = [
  { u: '@mizzix_42', init: 'M4', pct: 81, n: 80, val: 240, online: true },
  { u: '@kaalia_dt', init: 'KA', pct: 63, n: 62, val: 188, online: true },
  { u: '@urza_main', init: 'UR', pct: 47, n: 46, val: 142, online: false },
  { u: '@simic_ramp', init: 'SI', pct: 31, n: 31, val: 96, online: false },
];

const decklist = ref(
  '1x Vampiric Tutor\n1x Black Market Connections\n1x Reprieve\n1x Chronicle of Victory\n1x The Soul Stone\n1x Sire of Seven Deaths\n1x Emeritus of Woe',
);
</script>

<template>
  <div class="mx-auto max-w-[1180px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <!-- HEADER -->
    <div class="mb-4 flex flex-wrap items-center justify-between gap-3.5">
      <h2 class="font-display text-xl font-semibold tracking-tight">
        Cartes chez les autres joueurs
      </h2>
      <SegToggle v-model="mode" :options="modeOptions" />
    </div>

    <!-- MODE: PAR NOM -->
    <div v-if="mode === 'name'">
      <!-- Search bar -->
      <form class="mb-5 flex flex-wrap items-center gap-3" @submit.prevent="submitSearch">
        <div
          class="flex min-w-[240px] flex-1 items-center gap-2.5 rounded-2xl border border-slate-300 bg-black/20 py-2 pr-2 pl-4 transition-all duration-200 focus-within:border-cyan-500/40 focus-within:bg-black/10 focus-within:ring-4 focus-within:ring-cyan-500/10 dark:border-white/15 dark:focus-within:border-cyan-400/40"
        >
          <Icon
            name="lucide:search"
            size="20"
            class="flex-none text-slate-400 dark:text-slate-500"
          />
          <input
            v-model="q"
            placeholder="Nom de la carte…"
            class="min-w-0 flex-1 border-0 bg-transparent text-base text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
          />
          <button
            type="submit"
            class="inline-flex items-center justify-center gap-2 rounded-xl border border-transparent bg-cyan-500 px-4 py-2 text-sm leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
          >
            Chercher
          </button>
        </div>
        <button
          type="button"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-slate-200 bg-slate-100 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-slate-600 transition-all duration-150 select-none hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 md:hidden dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
          @click="sheet = true"
        >
          <Icon name="lucide:filter" :size="13" />
          Filtres
        </button>
      </form>

      <!-- BODY -->
      <div class="flex items-start gap-6">
        <!-- Sidebar filters (desktop) -->
        <aside
          class="sticky top-[86px] w-[210px] flex-none rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md max-md:hidden dark:border-white/10 dark:bg-zinc-900/60"
        >
          <CollectionFilters
            :active="active"
            :set-list="setList"
            :price-min="priceMin"
            :price-max="priceMax"
            :show-search="false"
            @toggle="toggle"
            @price-change="onPriceChange"
          />
        </aside>

        <!-- Main content -->
        <div class="min-w-0 flex-1">
          <div class="mb-3.5 flex min-h-[22px] items-center justify-between">
            <span v-if="submittedQ" class="text-sm text-slate-400 dark:text-slate-500">
              <b class="font-semibold text-slate-800 dark:text-slate-100"
                >{{ collectionData?.total ?? 0 }} résultat{{
                  (collectionData?.total ?? 0) > 1 ? 's' : ''
                }}</b
              >
              pour « {{ submittedQ }} »
            </span>
            <SegToggle v-model="size" :options="sizeOptions" size="sm" class="ml-auto" />
          </div>

          <!-- Loading state (initial) -->
          <div
            v-if="pending && allCards.length === 0"
            class="flex items-center justify-center py-16 font-mono text-sm text-slate-400 dark:text-slate-500"
          >
            <Icon name="lucide:loader-circle" :size="18" class="mr-2.5 animate-spin" />
            Chargement…
          </div>

          <!-- Empty state -->
          <div
            v-else-if="!pending && allCards.length === 0"
            class="flex flex-col items-center justify-center gap-4 py-20 text-slate-400 dark:text-slate-500"
          >
            <Icon name="lucide:search-x" :size="48" class="opacity-40" />
            <p class="text-center font-mono text-base">
              Aucune carte ne correspond à ta recherche.
            </p>
          </div>

          <!-- Grid -->
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
              <CardCell
                v-for="c in allCards"
                :key="c.scryfall_id"
                :scryfall-id="c.scryfall_id"
                :the-gatherer-id="c.the_gatherer_id ?? undefined"
                :name="c.name"
                :price="c.price_guide?.trend ?? 0"
                :foil="c.foil"
                :size="size"
                :owner-count="c.owner_count ?? undefined"
                @click="detail = c"
              />
            </div>

            <!-- Infinite scroll sentinel -->
            <div ref="sentinel" class="h-px" />
            <div
              v-if="pending && allCards.length > 0"
              class="flex items-center justify-center py-8 font-mono text-sm text-slate-400 dark:text-slate-500"
            >
              <Icon name="lucide:loader-circle" :size="16" class="mr-2 animate-spin" />
              Chargement…
            </div>
          </template>
        </div>
      </div>

      <!-- MOBILE FILTER SHEET -->
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
          <CollectionFilters
            :active="active"
            :set-list="setList"
            :price-min="priceMin"
            :price-max="priceMax"
            :show-search="false"
            @toggle="toggle"
            @price-change="onPriceChange"
          />
          <button
            class="mt-4 inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent bg-cyan-500 px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
            @click="sheet = false"
          >
            Voir les résultats
          </button>
        </div>
      </div>

      <!-- CARD DETAIL MODAL -->
      <CardDetailModal v-if="detail" :card="detail" @close="detail = null" />
    </div>

    <!-- MODE: PAR DECKLIST -->
    <div
      v-else
      class="grid [grid-template-columns:minmax(240px,320px)_1fr] items-start gap-6 max-md:[grid-template-columns:1fr]"
    >
      <!-- Left: paste zone -->
      <div
        class="flex flex-col gap-3 self-start rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <span
          class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
          >Coller ma decklist</span
        >
        <textarea
          v-model="decklist"
          rows="9"
          class="w-full resize-y p-3 font-mono text-xs leading-relaxed text-slate-800 dark:text-slate-100"
        />
        <div class="flex items-center justify-between">
          <span class="text-xs text-slate-400 dark:text-slate-500">99 cartes détectées</span>
          <span
            class="inline-flex cursor-default items-center gap-1.5 rounded-full border border-slate-200 bg-slate-100 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-slate-600 select-none dark:border-white/10 dark:bg-white/5 dark:text-slate-300"
          >
            <span class="h-2 w-2 rounded-full bg-violet-500 dark:bg-violet-400" /> 2 non reconnues
          </span>
        </div>
        <button
          class="inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent bg-cyan-500 px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
        >
          <Icon name="lucide:search" size="15" /> Trouver les joueurs
        </button>
      </div>

      <!-- Right: coverage results -->
      <div class="flex min-w-0 flex-1 flex-col gap-3.5">
        <div class="flex items-center justify-between">
          <h3 class="font-display text-base font-semibold tracking-tight">
            12 joueurs couvrent ta liste
          </h3>
          <button
            class="inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-cyan-500/30 bg-cyan-500/10 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-cyan-700 transition-all duration-150 select-none dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300"
          >
            % couverture <Icon name="lucide:chevron-down" size="13" />
          </button>
        </div>

        <div
          v-for="(c, i) in coverers"
          :key="c.u"
          :class="[
            'rounded-2xl p-4 shadow-lg backdrop-blur-md',
            i === 0
              ? 'border border-cyan-500/30 bg-cyan-500/10 dark:border-cyan-400/30 dark:bg-cyan-400/10'
              : 'border border-slate-200 bg-white/60 dark:border-white/10 dark:bg-zinc-900/60',
          ]"
        >
          <div class="mb-2.5 flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <PlayerAvatar :initials="c.init" :online="c.online" />
              <span
                class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
                >{{ c.u }}</span
              >
            </div>
            <span
              :class="[
                'font-mono font-bold tracking-tight whitespace-nowrap text-cyan-600 dark:text-cyan-400',
                i === 0 ? 'text-2xl' : 'text-xl',
              ]"
              >{{ c.pct }}%</span
            >
          </div>
          <div
            class="h-2 overflow-hidden rounded-full border border-slate-200 bg-black/30 dark:border-white/5"
          >
            <i
              class="block h-full rounded-full bg-cyan-500 transition-[width] duration-700 ease-out dark:bg-cyan-400"
              :style="{ width: c.pct + '%' }"
            />
          </div>
          <div class="mt-2.5 flex items-center justify-between">
            <span class="text-xs text-slate-400 dark:text-slate-500">
              couvre {{ c.n }}/99 cartes · valeur ≈
              <span class="font-mono tracking-tight">€{{ c.val }}</span>
            </span>
            <div v-if="i === 0" class="flex items-center gap-2">
              <button
                class="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-200 bg-transparent px-3 py-1.5 text-xs leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 max-md:hidden dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
              >
                Voir les {{ c.n }}
              </button>
              <NuxtLink
                to="/trade"
                class="inline-flex items-center justify-center gap-2 rounded-lg border border-transparent bg-cyan-500 px-3 py-1.5 text-xs leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
                >Composer l'échange</NuxtLink
              >
            </div>
            <NuxtLink
              v-else
              to="/trade"
              class="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-200 bg-transparent px-3 py-1.5 text-xs leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
              >Composer</NuxtLink
            >
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
