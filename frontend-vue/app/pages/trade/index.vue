<script setup lang="ts">
import type { TradeSummary } from '~/bindings/TradeSummary';

definePageMeta({ middleware: 'auth' });

const { listTrades } = useTradeService();

const params = ref({ page: 0, page_size: 20 });

const { data, pending, refresh } = await listTrades(params);

const items = ref<TradeSummary[]>([]);

watch(
  data,
  (d) => {
    if (!d) return;
    if (params.value.page === 0) {
      items.value = [...d.items];
    } else {
      items.value.push(...d.items);
    }
  },
  { immediate: true },
);

const moreResultsExist = computed(() =>
  data.value ? items.value.length < data.value.total : false,
);
const withinOffsetLimit = computed(() =>
  canLoadPage(params.value.page + 1, params.value.page_size, TRADES_MAX_OFFSET),
);
const hasMore = computed(() => moreResultsExist.value && withinOffsetLimit.value);
// Distinct from "no more results": there are more trades, but paging further would hit the
// backend's offset limit. Surfaced so the stop doesn't read as an empty/bugged list.
const offsetLimitReached = computed(() => moreResultsExist.value && !withinOffsetLimit.value);

const loadMore = () => {
  params.value.page += 1;
  refresh();
};

const formatUpdatedAt = (iso: string) =>
  new Date(iso).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', year: 'numeric' });
</script>

<template>
  <div class="mx-auto max-w-[760px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <h2 class="font-display mb-4 text-xl font-semibold tracking-tight">Mes échanges</h2>

    <!-- LOADING -->
    <div
      v-if="pending && items.length === 0"
      class="flex items-center justify-center py-16 font-mono text-sm text-slate-400 dark:text-slate-500"
    >
      <Icon name="lucide:loader-circle" size="18" class="mr-2.5 animate-spin" />
      Chargement…
    </div>

    <!-- EMPTY -->
    <div
      v-else-if="!pending && items.length === 0"
      class="flex flex-col items-center justify-center gap-4 py-20 text-slate-400 dark:text-slate-500"
    >
      <Icon name="lucide:arrow-left-right" :size="48" class="opacity-40" />
      <p class="text-center font-mono text-base">Aucun échange pour l’instant.</p>
      <NuxtLink
        to="/search"
        class="inline-flex items-center justify-center gap-2 rounded-xl border border-transparent bg-cyan-500 px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
        >Trouver un partenaire</NuxtLink
      >
    </div>

    <!-- LIST -->
    <template v-else>
      <div class="flex flex-col gap-2">
        <NuxtLink
          v-for="t in items"
          :key="t.id"
          :to="`/trade/${t.id}`"
          class="flex items-center gap-3 rounded-xl border border-slate-200 bg-white/60 px-3.5 py-3 shadow-lg backdrop-blur-md transition-all duration-150 hover:border-slate-300 hover:bg-white dark:border-white/10 dark:bg-zinc-900/60 dark:hover:border-white/15 dark:hover:bg-zinc-900"
        >
          <PlayerAvatar :initials="t.partner_username.slice(0, 2).toUpperCase()" />
          <div class="flex min-w-0 flex-1 flex-col gap-0.5">
            <span
              class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
              >{{ t.partner_username }}</span
            >
            <span class="text-xs text-slate-400 dark:text-slate-500">
              {{ t.my_card_count }} ↔ {{ t.partner_card_count }} cartes ·
              {{ formatUpdatedAt(t.updated_at) }}
            </span>
          </div>
          <TradeStatusPill :status="toTradeStatus(t.status)" size="sm" />
          <Icon
            name="lucide:chevron-right"
            size="16"
            class="flex-none text-slate-400 dark:text-slate-500"
          />
        </NuxtLink>
      </div>

      <div v-if="hasMore" class="mt-4 flex justify-center">
        <button
          class="inline-flex items-center justify-center gap-2 rounded-xl border border-slate-200 bg-transparent px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
          :disabled="pending"
          @click="loadMore"
        >
          <Icon v-if="pending" name="lucide:loader-circle" size="14" class="animate-spin" />
          Voir plus
        </button>
      </div>
      <p
        v-else-if="offsetLimitReached"
        class="mt-4 text-center font-mono text-sm text-slate-400 dark:text-slate-500"
      >
        Affichage limité aux {{ items.length }} échanges les plus récents.
      </p>
    </template>
  </div>
</template>
