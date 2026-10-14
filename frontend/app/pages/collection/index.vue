<script setup lang="ts">
import type { CollectionCard } from '~/bindings/CollectionCard';
import type { CardListScope } from '~/composables/useCardList';

import { useRoute } from 'nuxt/app';

definePageMeta({ middleware: 'auth' });

const {
  getCollection,
  getCollectionStats,
  importCards,
  getCardImport,
  listCardImports,
  getPriceHistory,
} = useCollectionService();

// The text filter is read back from the URL (reload, bookmark), like the criteria of the list.
const restoredQ = useRoute().query.q;
const q = ref(typeof restoredQ === 'string' && restoredQ.trim() ? restoredQ : '');
const qDebounced = refDebounced(q, 200);

const size = usePreference(CARD_SIZE_PREF);
// Detail view is desktop-only — the extra KPIs/axis/range chips don't fit a small screen.
const isDesktop = useMediaQuery('(min-width: 768px)');
const pageSize = useCardPageSize(size, isDesktop);

// Same key and options as every other reader of the stats, so they share one cache entry.
const { data: statsData } = await useAsyncData('collection-stats', () => getCollectionStats(), {
  lazy: true,
});

const setList = computed(() => statsData.value?.sets ?? []);

const priceMin = computed(() =>
  statsData.value?.price_trend_min != null ? Math.floor(statsData.value.price_trend_min / 100) : 0,
);
const priceMax = computed(() =>
  statsData.value?.price_trend_max != null ? Math.ceil(statsData.value.price_trend_max / 100) : 150,
);

const list = reactive(
  useCardList<CollectionCard, { q: string }>({
    context: COLLECTION_CRITERIA,
    fetchPage: getCollection,
    // The scope of the collection: its text filter (debounced), which its URL carries too.
    scope: (): CardListScope<{ q: string }> => {
      const text = qDebounced.value;
      return { params: { q: text }, query: text.trim() ? { q: text } : {} };
    },
    pageSize,
    priceMin,
    priceMax,
  }),
);

const graph = usePreference(COLLECTION_GRAPH_PREF);
const showDetail = computed(() => graph.value === 'expanded' && isDesktop.value);
const graphRange = usePreference(COLLECTION_GRAPH_RANGE_PREF);

const {
  data: priceHistoryData,
  pending: priceHistoryPending,
  refresh: refreshPriceHistory,
} = await useAsyncData(
  'collection-price-history',
  () => getPriceHistory(periodDateRange(graphRange.value)),
  { lazy: true },
);

watch(graphRange, () => refreshPriceHistory());

const priceHistory = computed(() => priceHistoryData.value ?? []);
const trend = computed(() => priceTrend(priceHistory.value));

const importOpen = ref(false);
const importLoading = ref(false);
const isDragging = ref(false);
const fileInputRef = ref<HTMLInputElement | null>(null);
const {
  step: importStep,
  status: importStatus,
  error: importError,
  wasAlreadyRunning: importWasAlreadyRunning,
  progressPercent: importProgressPercent,
  failureMessage: importFailureMessage,
  start: startImportFlow,
  stopPolling: stopImportPolling,
  reset: resetImportFlow,
} = useCardImportFlow(
  { importCards, getCardImport, listCardImports },
  { onCompleted: list.reload },
);
const detail = ref<CollectionCard | null>(null);

const graphOptions = [
  {
    value: 'compact',
    label: '',
    icon: 'lucide:bar-chart-2',
    title: 'Graphe compact',
    tone: 'primary',
  },
  {
    value: 'expanded',
    label: '',
    icon: 'lucide:trending-up',
    title: 'Graphe étendu',
    tone: 'primary',
  },
];
const sizeOptions = [
  { value: 'sm', label: '', icon: 'lucide:grid-3x3', title: 'Petites cartes', tone: 'primary' },
  { value: 'md', label: '', icon: 'lucide:grid-2x2', title: 'Cartes moyennes', tone: 'primary' },
  { value: 'lg', label: '', icon: 'lucide:square', title: 'Grandes cartes', tone: 'primary' },
];

onBeforeUnmount(stopImportPolling);

const openImport = () => {
  resetImportFlow();
  importOpen.value = true;
};

const closeImport = () => {
  stopImportPolling();
  importOpen.value = false;
};

const handleFile = async (file: File) => {
  if (!file.name.endsWith('.csv')) {
    importError.value = 'Le fichier doit être au format .csv';
    return;
  }
  importLoading.value = true;
  try {
    const csv = await file.text();
    await startImportFlow(csv);
  } finally {
    importLoading.value = false;
    if (fileInputRef.value) fileInputRef.value.value = '';
  }
};

const onFileInputChange = (e: Event) => {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (file) handleFile(file);
};

const onDrop = (e: DragEvent) => {
  e.preventDefault();
  isDragging.value = false;
  const file = e.dataTransfer?.files[0];
  if (file) handleFile(file);
};

const onDragOver = (e: DragEvent) => {
  e.preventDefault();
  isDragging.value = true;
};

const onDragLeave = () => {
  isDragging.value = false;
};
</script>

<template>
  <div class="mx-auto max-w-[1180px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <!-- ── VALUE BAR — immersive (compact) ⇄ framed + scales (detail) ── -->
    <div
      :class="[
        'relative mb-4 overflow-hidden rounded-2xl border border-slate-200 bg-white/60 shadow-lg backdrop-blur-md transition-[height] duration-[450ms] ease-[cubic-bezier(0.4,0,0.1,1)] dark:border-white/10 dark:bg-zinc-900/60',
        showDetail ? 'h-[220px] md:h-[396px]' : 'h-[188px] max-md:h-[220px]',
      ]"
    >
      <!-- graph background -->
      <div
        :class="[
          'absolute transition-[inset] duration-[450ms] ease-[cubic-bezier(0.4,0,0.1,1)]',
          showDetail ? 'inset-0 md:inset-x-4 md:top-[100px] md:bottom-[52px]' : 'inset-0',
        ]"
      >
        <PriceTrendGraph
          :entries="priceHistory"
          :pending="priceHistoryPending"
          :detail="showDetail"
        />
      </div>

      <!-- scrim (fades out in detail mode) -->
      <div
        :class="[
          'pointer-events-none absolute inset-0 z-[1] bg-gradient-to-b from-white/95 via-white/55 to-transparent transition-opacity duration-300 dark:from-zinc-900/95 dark:via-zinc-900/55',
          showDetail ? 'opacity-0' : 'opacity-100',
        ]"
      />

      <!-- top: KPIs + controls -->
      <div
        class="absolute inset-x-0 top-0 z-[3] flex flex-col items-start justify-between gap-3 px-5 py-5 max-md:gap-2.5 md:flex-row md:flex-wrap"
      >
        <div class="flex items-stretch">
          <div class="flex flex-col gap-1">
            <span
              class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
              >Valeur totale · CardMarket</span
            >
            <span
              class="font-mono text-[clamp(28px,3.4vw,36px)] font-bold tracking-tight whitespace-nowrap"
              >{{ formatPrice(trend.lastTrendCents) }}</span
            >
          </div>
          <div
            class="ml-5 hidden flex-col justify-center gap-1 border-l border-slate-200 pl-5 md:flex dark:border-white/10"
          >
            <span
              class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
              >Variation · {{ graphRange }}</span
            >
            <span
              :class="[
                'font-mono text-lg font-semibold',
                trend.variation.positive ? 'text-primary' : 'text-red-500 dark:text-red-400',
              ]"
              >{{ trend.variation.positive ? '▴' : '▾' }} {{ trend.variation.pctLabel }}</span
            >
          </div>
          <div
            v-if="statsData"
            class="ml-5 hidden flex-col justify-center gap-1 border-l border-slate-200 pl-5 md:flex dark:border-white/10"
          >
            <span
              class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
              >Cartes</span
            >
            <span class="font-mono text-lg font-semibold">{{
              statsData.total_cards.toLocaleString('fr-FR')
            }}</span>
          </div>
          <div
            v-if="statsData"
            class="ml-5 hidden flex-col justify-center gap-1 border-l border-slate-200 pl-5 md:flex dark:border-white/10"
          >
            <span
              class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
              >Uniques</span
            >
            <span class="font-mono text-lg font-semibold">{{ statsData.unique_cards }}</span>
          </div>
        </div>

        <div class="hidden items-center gap-2.5 md:flex">
          <div v-if="showDetail" class="flex gap-1.5">
            <button
              v-for="r in GRAPH_RANGES"
              :key="r"
              :class="[
                'inline-flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap transition-all duration-150 select-none',
                graphRange === r
                  ? 'border-primary/30 bg-primary/10 text-primary-ink'
                  : 'border-slate-200 bg-slate-100 text-slate-600 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100',
              ]"
              @click="graphRange = r"
            >
              {{ r }}
            </button>
          </div>
          <SegToggle v-model="graph" :options="graphOptions" size="sm" />
        </div>
      </div>

      <!-- bottom-left: sync status (hidden in detail, hidden on mobile — no room next to legend) -->
      <span
        :class="[
          'text-2xs absolute bottom-3.5 left-5 z-[3] hidden items-center gap-1.5 font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase transition-opacity duration-300 sm:flex dark:text-slate-500',
          showDetail ? 'pointer-events-none opacity-0' : 'opacity-100',
        ]"
      >
        <Icon name="lucide:refresh-cw" :size="11" /> synchro auto · il y a 2 h
      </span>

      <!-- bottom-right: legend -->
      <div class="absolute right-5 bottom-3.5 z-[3] flex gap-4">
        <span
          class="text-2xs flex items-center gap-1.5 font-mono whitespace-nowrap text-slate-500 dark:text-slate-300"
        >
          <span class="border-primary/40 bg-primary/25 h-2.5 w-[15px] rounded-[3px] border" />
          low → avg
        </span>
        <span
          class="text-2xs flex items-center gap-1.5 font-mono whitespace-nowrap text-slate-500 dark:text-slate-300"
        >
          <span class="border-primary h-0 w-[17px] border-t-[2.5px]" />
          trend
        </span>
      </div>
    </div>

    <div
      class="focus-within:border-primary/40 focus-within:ring-primary/20 mb-4 flex items-center gap-2.5 rounded-2xl border border-slate-400/50 bg-black/5 py-2 pr-4 pl-4 transition-all duration-200 focus-within:bg-black/5 focus-within:ring-4 dark:border-white/15 dark:bg-black/30"
    >
      <Icon name="lucide:search" size="20" class="flex-none text-slate-400 dark:text-slate-500" />
      <input
        v-model="q"
        placeholder="Filtrer ma collection…"
        class="min-w-0 flex-1 border-0 bg-transparent text-base text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
      />
    </div>

    <button
      class="bg-primary hover:bg-primary-soft mb-4 inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0 md:hidden"
      @click="openImport"
    >
      <Icon name="lucide:upload" :size="16" />
      Importer Manabox
    </button>

    <CardListShell
      :list="list"
      :set-list="setList"
      :price-min="priceMin"
      :price-max="priceMax"
      :size="size"
      :overlay-open="!!detail || importOpen"
    >
      <template #sidebar>
        <button
          class="bg-primary hover:bg-primary-soft inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0"
          @click="openImport"
        >
          <Icon name="lucide:upload" :size="16" />
          Importer Manabox
        </button>
      </template>

      <template #toolbar="{ openFilters }">
        <button
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-slate-200 bg-slate-100 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-slate-600 transition-all duration-150 select-none hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 md:hidden dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
          @click="openFilters"
        >
          <Icon name="lucide:filter" :size="13" />
          Filtres
        </button>
        <div class="ml-auto flex items-center gap-2.5">
          <SegToggle v-model="size" :options="sizeOptions" size="sm" class="max-md:hidden" />
          <SortToggle
            :model-value="list.criteria"
            :options="list.sorts"
            @update:model-value="list.setSort"
          />
        </div>
      </template>

      <template #empty>
        <Icon name="lucide:inbox" :size="48" class="opacity-40" />
        <p class="text-center font-mono text-base">Aucune carte dans la collection.</p>
        <button
          class="mt-1 flex cursor-pointer items-center gap-2 rounded-xl bg-slate-100 px-4 py-2 font-mono text-sm transition-colors hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700"
          @click="openImport()"
        >
          <Icon name="lucide:upload" :size="14" />
          Importer depuis Manabox
        </button>
      </template>

      <template #cell="{ card }">
        <CardCell
          :image-url="card.image_url"
          :name="card.name"
          :qty="card.collection_entry?.quantity ?? 0"
          :purchased="card.collection_entry?.purchase_price ?? 0"
          :trend="card.price_guide?.trend ?? 0"
          deal="compare"
          :foil="card.foil"
          :size="size"
          :reserved="card.reserved"
          @click="detail = card"
        />
      </template>

      <template #limit>
        Affichage limité aux {{ list.cards.length }} premières cartes — affine tes filtres pour voir
        le reste.
      </template>
    </CardListShell>

    <CardDetailModal v-if="detail" :card="detail" @close="detail = null" />

    <div
      v-if="importOpen"
      class="fixed inset-0 z-[80] grid animate-[fade_0.2s_ease] place-items-center bg-black/60 p-5 backdrop-blur-sm"
      @click="closeImport"
    >
      <div
        class="w-full max-w-[480px] animate-[pop_0.26s_cubic-bezier(0.3,1.2,0.4,1)] rounded-3xl border border-slate-300 bg-white p-6 shadow-2xl dark:border-white/15 dark:bg-zinc-900"
        @click.stop
      >
        <div class="mb-1 flex items-center justify-between">
          <h3 class="font-display text-xl font-semibold tracking-tight">Importer depuis Manabox</h3>
          <button
            class="hover:ring-primary/10 grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 hover:ring-4 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
            @click="closeImport"
          >
            <Icon name="lucide:x" :size="16" />
          </button>
        </div>

        <template v-if="importStep === 'idle'">
          <p class="mt-0 text-sm text-slate-600 dark:text-slate-300">
            Exporte ta collection en
            <span class="font-mono tracking-tight">.csv</span>
            depuis Manabox, puis dépose-la ici.
          </p>
          <input
            ref="fileInputRef"
            type="file"
            accept=".csv"
            class="hidden"
            @change="onFileInputChange"
          />
          <div
            class="mt-2 flex cursor-pointer flex-col items-center gap-3 rounded-xl border-[1.5px] border-dashed p-8 transition-all duration-200"
            :class="
              isDragging
                ? 'border-primary/40 bg-primary/10'
                : 'hover:border-primary/40 hover:bg-primary/10 border-slate-300 bg-black/10 dark:border-white/15'
            "
            @click="fileInputRef?.click()"
            @drop="onDrop"
            @dragover="onDragOver"
            @dragleave="onDragLeave"
          >
            <Icon
              :name="importLoading ? 'lucide:loader-2' : 'lucide:upload'"
              :size="30"
              class="text-primary"
              :class="{ 'animate-spin': importLoading }"
            />
            <div class="flex flex-col items-center gap-1">
              <span class="font-semibold">{{
                importLoading ? 'Envoi en cours…' : 'Glisse ton fichier .csv ici'
              }}</span>
              <span v-if="!importLoading" class="text-xs text-slate-400 dark:text-slate-500"
                >ou clique pour parcourir</span
              >
            </div>
          </div>
          <p v-if="importError" class="mt-2.5 mb-0 text-sm text-red-400">
            {{ importError }}
          </p>
        </template>

        <template v-else-if="importStep === 'progress'">
          <p v-if="importWasAlreadyRunning" class="mt-0 text-sm text-amber-500">
            Un import est déjà en cours pour ton compte — voici sa progression :
          </p>
          <p v-else class="mt-0 text-sm text-slate-600 dark:text-slate-300">Import en cours…</p>
          <div
            class="mt-4 flex items-center justify-between text-sm text-slate-600 dark:text-slate-300"
          >
            <span
              >{{ importStatus?.processed_lines ?? 0 }} /
              {{ importStatus?.total_lines ?? 0 }} cartes</span
            >
            <span class="font-mono">{{ importProgressPercent }}%</span>
          </div>
          <div
            class="mt-2 h-2.5 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-white/10"
            role="progressbar"
            :aria-valuenow="importProgressPercent"
            aria-valuemin="0"
            aria-valuemax="100"
          >
            <div
              class="bg-primary h-full rounded-full transition-[width] duration-300"
              :style="{ width: importProgressPercent + '%' }"
            />
          </div>
        </template>

        <template v-else-if="importStep === 'done' && importStatus?.status === 'completed'">
          <p class="mt-0 text-sm text-slate-600 dark:text-slate-300">
            <strong>{{ importStatus.total_lines }}</strong> carte(s) importée(s)
            <template v-if="importStatus.error_count > 0">
              — <strong>{{ importStatus.error_count }}</strong> ligne(s) ignorée(s)
            </template>
          </p>
          <ul
            v-if="importStatus.errors.length"
            class="mt-2 max-h-32 list-disc space-y-1 overflow-y-auto pl-5 text-xs text-amber-600 dark:text-amber-400"
          >
            <li v-for="e in importStatus.errors" :key="e.line">
              {{ importLineErrorMessage(e) }}
            </li>
          </ul>
          <button
            class="bg-primary hover:bg-primary-soft mt-4 inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0"
            @click="closeImport"
          >
            Fermer
          </button>
        </template>

        <!-- Step: done (failed) -->
        <template v-else-if="importStep === 'done'">
          <p class="mt-0 text-sm text-red-400">
            {{ importFailureMessage }}
          </p>
          <button
            class="bg-primary hover:bg-primary-soft mt-4 inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0"
            @click="openImport"
          >
            Réessayer
          </button>
        </template>
      </div>
    </div>
  </div>
</template>
