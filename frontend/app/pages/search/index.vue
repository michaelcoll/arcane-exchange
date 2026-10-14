<script setup lang="ts">
import type { CollectionCard } from '~/bindings/CollectionCard';
import type { UserSuggestion } from '~/bindings/UserSuggestion';
import type { CardListScope } from '~/composables/useCardList';
import type { SearchListParams, SearchMode } from '~/utils/search-url';

import { useRoute } from 'nuxt/app';

definePageMeta({ middleware: 'auth' });

const { getCollectionStats } = useCollectionService();
const { getSearch, getSearchSets } = useSearchService();
const { autocompleteUsers } = useAutocompleteService();

// The search (mode, name, player, text filter), read back from the URL: home page, trade, reload.
const search = reactive(parseSearchQuery(useRoute().query));

const modeLabels: Record<SearchMode, { label: string; tone: string; kbd: string }> = {
  name: { label: 'Par nom', tone: 'primary', kbd: '1' },
  decklist: { label: 'Par decklist', tone: 'primary', kbd: '2' },
  player: { label: 'Par joueur', tone: 'secondary', kbd: '3' },
};
const modeOptions = SEARCH_MODES.map((value) => ({ value, ...modeLabels[value] }));

/* ---------- MODE: PAR NOM ---------- */
const q = ref(search.q ?? '');
watchDebounced(q, (v) => (search.q = v), { debounce: 300 });

const searchAreaRef = ref<HTMLElement | null>(null);

const focusSearchField = () => {
  const root = searchAreaRef.value;
  if (!root) return;
  const el = root.querySelector<HTMLElement>(
    '[data-autofocus], input:not([readonly]):not([type="range"]), textarea',
  );
  el?.focus({ preventScroll: true });
};

const size = usePreference(CARD_SIZE_PREF);
const isDesktop = useMediaQuery('(min-width: 768px)');
const pageSize = useCardPageSize(size, isDesktop);

/* ---------- MODE: PAR JOUEUR ---------- */
const playerFilterQ = ref(search.filter ?? '');
watchDebounced(playerFilterQ, (v) => (search.filter = v), { debounce: 300 });

const openPlayer = (username: string) => {
  playerFilterQ.value = '';
  Object.assign(search, { mode: 'player', player: username, filter: undefined });
};

const clearPlayer = () => {
  search.player = undefined;
};

// Leaving the player mode leaves the player, and starts the search by name afresh.
const setMode = (mode: string) => {
  if (!isSearchMode(mode)) return;
  if (search.mode === 'player' && mode !== 'player') {
    clearPlayer();
    q.value = '';
    search.q = undefined;
  }
  search.mode = mode;
};

// The profile only completes the header (card count, Note): the list is loaded on the username.
const profile = ref<UserSuggestion>();
watch(
  () => search.player,
  async (username) => {
    profile.value = undefined;
    if (!username) return;
    // Ignored on purpose: without the profile the header only loses its card count and its Note,
    // and the player's cards still load.
    const suggestions = await autocompleteUsers(username).catch(() => []);
    if (search.player !== username) return;
    profile.value = suggestions.find((u) => u.username.toLowerCase() === username.toLowerCase());
  },
  { immediate: true },
);

const pageTitle = computed(() =>
  search.player ? `Collection de @${search.player}` : 'Cartes chez les autres joueurs',
);

watch(
  () => [search.mode, !!search.player],
  () => nextTick(focusSearchField),
);

// Same key and options as every other reader of the stats, so they share one cache entry.
const { data: statsData } = await useAsyncData('collection-stats', () => getCollectionStats(), {
  lazy: true,
});

// The price range still comes from my own collection (`statsData`), to be settled later.
const priceMin = computed(() =>
  statsData.value?.price_trend_min != null ? Math.floor(statsData.value.price_trend_min / 100) : 0,
);
const priceMax = computed(() =>
  statsData.value?.price_trend_max != null ? Math.ceil(statsData.value.price_trend_max / 100) : 150,
);

const list = reactive(
  useCardList<CollectionCard, SearchListParams>({
    context: () => searchCriteriaContext(search),
    fetchPage: getSearch,
    scope: (): CardListScope<SearchListParams> => ({
      params: searchListParams(search),
      query: toSearchQuery(search),
    }),
    pageSize,
    priceMin,
    priceMax,
  }),
);

// The sets offered are those of the cards the search can return (submitted name, or player
// alone), not those of my collection. An active set read from the URL stays applied even when
// this list does not offer it.
const { data: setList } = await getSearchSets(() => searchSetsScope(search));

onMounted(() => {
  nextTick(focusSearchField);

  // The decklist text is read back from sessionStorage, and left there.
  if (search.mode === 'decklist') {
    const saved = loadDecklist();
    if (saved !== null) decklist.value = saved;
  }
});

const detail = ref<CollectionCard | null>(null);

const sizeOptions = [
  { value: 'sm', label: '', icon: 'lucide:grid-3x3', title: 'Petites cartes', tone: 'primary' },
  { value: 'md', label: '', icon: 'lucide:grid-2x2', title: 'Cartes moyennes', tone: 'primary' },
  { value: 'lg', label: '', icon: 'lucide:square', title: 'Grandes cartes', tone: 'primary' },
];

/* ---------- MODE: PAR DECKLIST ---------- */
const coverers = [
  { u: 'mizzix_42', pct: 81, n: 80, val: 240 },
  { u: 'kaalia_dt', pct: 63, n: 62, val: 188 },
  { u: 'urza_main', pct: 47, n: 46, val: 142 },
  { u: 'simic_ramp', pct: 31, n: 31, val: 96 },
];

const decklist = ref(
  '1x Vampiric Tutor\n1x Black Market Connections\n1x Reprieve\n1x Chronicle of Victory\n1x The Soul Stone\n1x Sire of Seven Deaths\n1x Emeritus of Woe',
);

// From the (mocked) decklist section: straight to that player's collection.
const goToPlayerCollection = (username: string) => openPlayer(username.replace(/^@/, ''));
</script>

<template>
  <div class="mx-auto max-w-[1180px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <div class="mb-4 flex flex-wrap items-center justify-between gap-3.5">
      <h2 class="font-display text-xl font-semibold tracking-tight">
        {{ pageTitle }}
      </h2>
      <SegToggle
        :model-value="search.mode"
        :options="modeOptions"
        shortcuts
        @update:model-value="setMode"
      />
    </div>

    <div v-if="search.mode === 'name' || search.player">
      <div
        v-if="search.player"
        class="border-secondary/30 dark:border-secondary/20 dark:bg-secondary/5 mb-5 flex flex-wrap items-center gap-4 rounded-2xl border bg-white/60 p-4 shadow-lg backdrop-blur-md"
      >
        <PlayerAvatar :username="search.player" size="lg" />
        <div class="flex min-w-0 flex-1 flex-col gap-1">
          <span class="text-lg font-semibold text-slate-800 dark:text-slate-100"
            ><span class="text-slate-400 dark:text-slate-500">@</span>{{ search.player }}</span
          >
          <div
            v-if="profile"
            class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-slate-400 dark:text-slate-500"
          >
            <span>{{ profile.card_count }} cartes</span>
            <span class="text-secondary inline-flex items-center gap-1">
              <Icon name="lucide:star" :size="13" />{{ profile.note.toFixed(1).replace('.', ',') }}
            </span>
          </div>
        </div>
        <button
          class="inline-flex items-center justify-center gap-2 rounded-xl border border-solid border-slate-200 bg-transparent px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 max-md:w-full dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
          @click="clearPlayer"
        >
          <Icon name="lucide:repeat" :size="14" /> Changer de joueur
        </button>
      </div>

      <div
        v-if="search.mode === 'name'"
        ref="searchAreaRef"
        class="mb-5 flex flex-wrap items-center gap-3"
      >
        <div
          class="focus-within:border-primary/40 focus-within:ring-primary/10 flex min-w-[240px] flex-1 items-center gap-2.5 rounded-2xl border border-slate-300 bg-black/20 py-2 pr-4 pl-4 transition-all duration-200 focus-within:bg-black/10 focus-within:ring-4 dark:border-white/15"
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
        </div>
      </div>

      <!-- Filter bar (mode: par joueur) -->
      <div v-else class="mb-5 flex flex-wrap items-center gap-3">
        <div
          class="0 focus-within:border-primary/40 focus-within:ring-primary/20 flex min-w-[240px] flex-1 items-center gap-2.5 rounded-2xl border border-slate-400/50 bg-black/5 py-2 pr-4 pl-4 transition-all duration-200 focus-within:bg-black/5 focus-within:ring-4 dark:border-white/15 dark:bg-black/30"
        >
          <Icon
            name="lucide:search"
            size="20"
            class="flex-none text-slate-400 dark:text-slate-500"
          />
          <input
            v-model="playerFilterQ"
            data-autofocus
            placeholder="Filtrer dans sa collection…"
            class="min-w-0 flex-1 border-0 bg-transparent text-base text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
          />
        </div>
      </div>

      <CardListShell
        :list="list"
        :set-list="setList"
        :price-min="priceMin"
        :price-max="priceMax"
        :size="size"
        :overlay-open="!!detail"
      >
        <template #toolbar="{ openFilters }">
          <span
            v-if="search.player"
            class="text-sm text-slate-400 max-md:hidden dark:text-slate-500"
          >
            <b class="font-semibold text-slate-800 dark:text-slate-100"
              >{{ list.total }} carte{{ list.total > 1 ? 's' : '' }}</b
            >
            visible{{ list.total > 1 ? 's' : '' }}
            <template v-if="profile">sur {{ profile.card_count }}</template>
          </span>
          <span
            v-else-if="search.q"
            class="text-sm text-slate-400 max-md:hidden dark:text-slate-500"
          >
            <b class="font-semibold text-slate-800 dark:text-slate-100"
              >{{ list.total }} résultat{{ list.total > 1 ? 's' : '' }}</b
            >
            pour « {{ search.q }} »
          </span>
          <button
            type="button"
            class="inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-slate-200 bg-slate-300 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-slate-600 transition-all duration-150 select-none hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 md:hidden dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
            @click="openFilters"
          >
            <Icon name="lucide:filter" :size="13" />
            Filtres
          </button>
          <SortToggle
            :model-value="list.criteria"
            :options="list.sorts"
            class="ml-auto"
            @update:model-value="list.setSort"
          />
          <SegToggle v-model="size" :options="sizeOptions" size="sm" class="max-md:hidden" />
        </template>

        <template #empty>
          <Icon name="lucide:search-x" :size="48" class="opacity-40" />
          <p class="text-center font-mono text-base">Aucune carte ne correspond à ta recherche.</p>
        </template>

        <template #cell="{ card }">
          <CardCell
            :image-url="card.image_url"
            :name="card.name"
            :price="card.price_guide?.trend ?? 0"
            :foil="card.foil"
            :size="size"
            :owner-count="card.owner_count ?? undefined"
            :reserved="card.reserved"
            @click="detail = card"
          />
        </template>

        <template #limit>
          Affichage limité aux {{ list.cards.length }} premiers résultats — affine ta recherche pour
          voir le reste.
        </template>
      </CardListShell>

      <CardDetailModal v-if="detail" :card="detail" @close="detail = null" />
    </div>

    <!-- MODE: PAR JOUEUR (pas encore de joueur choisi) -->
    <div
      v-else-if="search.mode === 'player'"
      ref="searchAreaRef"
      class="mx-auto mt-8 flex max-w-[560px] flex-col items-center gap-3.5 rounded-2xl border border-slate-200 bg-white/60 p-6 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
    >
      <span
        class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Quel joueur veux-tu parcourir ?</span
      >
      <PlayerPicker
        :model-value="null"
        cta="Voir ses cartes"
        @update:model-value="(p) => p && openPlayer(p.username)"
      />
      <span class="text-center text-xs text-slate-400 dark:text-slate-500">
        Tu verras sa collection complète, filtrable comme la tienne.
      </span>
    </div>

    <!-- MODE: PAR DECKLIST -->
    <div
      v-else
      class="grid [grid-template-columns:minmax(240px,320px)_1fr] items-start gap-6 max-md:[grid-template-columns:1fr]"
    >
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
            <span class="bg-secondary h-2 w-2 rounded-full" /> 2 non reconnues
          </span>
        </div>
        <button
          class="bg-primary hover:bg-primary-soft inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0"
          @click="saveDecklist(decklist)"
        >
          <Icon name="lucide:search" size="15" /> Trouver les joueurs
        </button>
      </div>

      <div class="flex min-w-0 flex-1 flex-col gap-3.5">
        <div class="flex items-center justify-between">
          <h3 class="font-display text-base font-semibold tracking-tight">
            12 joueurs couvrent ta liste
          </h3>
          <button
            class="border-primary/30 bg-primary/10 text-primary-ink inline-flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap transition-all duration-150 select-none"
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
              ? 'border-primary/30 bg-primary/10 border'
              : 'border border-slate-200 bg-white/60 dark:border-white/10 dark:bg-zinc-900/60',
          ]"
        >
          <div class="mb-2.5 flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <PlayerAvatar :username="c.u" />
              <span
                class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
                ><span class="text-slate-400 dark:text-slate-500">@</span>{{ c.u }}</span
              >
            </div>
            <span
              :class="[
                'text-primary font-mono font-bold tracking-tight whitespace-nowrap',
                i === 0 ? 'text-2xl' : 'text-xl',
              ]"
              >{{ c.pct }}%</span
            >
          </div>
          <div
            class="h-2 overflow-hidden rounded-full border border-slate-200 bg-black/30 dark:border-white/5"
          >
            <i
              class="bg-primary block h-full rounded-full transition-[width] duration-700 ease-out"
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
              <button
                class="bg-primary hover:bg-primary-soft inline-flex items-center justify-center gap-2 rounded-lg border border-transparent px-3 py-1.5 text-xs leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0"
                @click="goToPlayerCollection(c.u)"
              >
                Composer l'échange
              </button>
            </div>
            <button
              v-else
              class="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-200 bg-transparent px-3 py-1.5 text-xs leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
              @click="goToPlayerCollection(c.u)"
            >
              Composer
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
