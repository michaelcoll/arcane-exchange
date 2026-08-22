<script setup lang="ts">
import type { TradeStatusParam } from '~/bindings/TradeStatusParam';
import type { UserSuggestion } from '~/bindings/UserSuggestion';

const mode = ref<'name' | 'decklist' | 'player'>('name');
const q = ref('');

type Player = UserSuggestion;
const player = ref<Player | null>(null);

const searchAreaRef = ref<HTMLElement | null>(null);

const focusSearchField = () => {
  const root = searchAreaRef.value;
  if (!root) return;
  const el = root.querySelector<HTMLElement>(
    '[data-autofocus], input:not([readonly]):not([type="range"]), textarea',
  );
  el?.focus({ preventScroll: true });
};

onMounted(() => nextTick(focusSearchField));
watch(
  () => [mode.value, !!player.value],
  () => nextTick(focusSearchField),
);

const { recents, addRecentSearch } = useRecentSearches(CARD_RECENT_SEARCHES_KEY);

const trends = [
  {
    name: 'Sire of Seven Deaths',
    color: 'm',
    price: 3100,
    id: '8d8432a7-1c8a-4cfb-947c-ecf9791063eb',
  },
  { name: 'Vampiric Tutor', color: 'b', price: 2800, id: '7a79190f-de60-4eb6-b925-594eb76ca8c3' },
  {
    name: 'Chronicle of Victory',
    color: 'w',
    price: 2400,
    id: 'b3c2d68d-690b-41e7-99ed-2d20c7e0a9b4',
  },
  { name: 'The Soul Stone', color: 'b', price: 900, id: '1982f910-a9bd-4e94-a187-84381b22aacc' },
  {
    name: 'Black Market Connections',
    color: 'b',
    price: 1300,
    id: '318f8ec3-0613-448d-87d2-0bcc9e95da64',
  },
];

const { getCollectionStats, getPriceHistory } = useCollectionService();
const { data: collectionStatsData } = await getCollectionStats();

const collectionHistoryParams = ref(lastNDaysRange(30));
const { data: collectionHistoryData, pending: collectionHistoryPending } = await getPriceHistory(
  collectionHistoryParams,
  'home-collection-price-history',
);

const collectionEnvelopeData = computed(() => toEnvelopeData(collectionHistoryData.value ?? []));
const hasEnoughCollectionHistory = computed(() => collectionEnvelopeData.value.length >= 2);
const collectionTotalValueCents = computed(() => {
  const entries = collectionHistoryData.value;
  return entries && entries.length > 0 ? entries[entries.length - 1]!.trend : 0;
});
const collectionVariation = computed(() => computeVariation(collectionHistoryData.value ?? []));

const { listTrades } = useTradeService();
const activeTradesParams = {
  page: 0,
  page_size: 3,
  status: ['PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED'] as TradeStatusParam[],
};
const { data: activeTradesData, pending: activeTradesPending } =
  await listTrades(activeTradesParams);
const activeTrades = computed(() => activeTradesData.value?.items ?? []);
const activeTradesTotal = computed(
  () => activeTradesData.value?.total ?? activeTrades.value.length,
);

const searchOptions = [
  { value: 'name', label: 'Nom de carte', tone: 'cyan', kbd: '1' },
  { value: 'decklist', label: 'Decklist', tone: 'cyan', kbd: '2' },
  { value: 'player', label: 'Joueur', tone: 'vio', kbd: '3' },
];

const goToPlayer = (p: Player) => {
  navigateTo(`/search?player=${encodeURIComponent(p.username)}`);
};

const selectRecent = (name: string) => {
  navigateToSearch(name, 'name');
};

const decklistContent = ref('');

const navigateToSearch = (searchQ: string, searchMode: string) => {
  if (searchMode === 'name' && searchQ.trim()) {
    addRecentSearch(searchQ);
  }
  const params = new URLSearchParams();
  if (searchQ.trim()) params.set('q', searchQ.trim());
  if (searchMode) params.set('mode', searchMode);
  const query = params.toString();
  navigateTo(query ? `/search?${query}` : '/search');
};

const handleDecklistSearch = () => {
  if (decklistContent.value.trim()) {
    try {
      sessionStorage.setItem('tae_decklist_pending', decklistContent.value);
    } catch {
      // Log en cas de contenu trop volumineux (>50 ko)
      if (decklistContent.value.length > 50_000) {
        console.warn('Decklist très volumineuse (>50 ko)');
      }
    }
  }
  navigateTo('/search');
};
</script>

<template>
  <div class="mx-auto max-w-[1180px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <!-- ── HERO ── -->
    <div class="flex flex-col items-center gap-6 pt-10 pb-8 text-center">
      <!-- Hero logo lockup -->
      <div class="flex flex-col items-center gap-2.5">
        <div class="flex items-center gap-[clamp(12px,2.4vw,20px)]">
          <span
            class="font-display text-[clamp(30px,5.2vw,46px)] leading-none font-semibold tracking-tight text-slate-800 dark:text-slate-100"
            >Arcane</span
          >
          <span class="grid flex-none place-items-center" aria-hidden="true">
            <svg
              viewBox="0 0 28 28"
              fill="none"
              stroke-width="2.2"
              stroke-linejoin="round"
              class="h-[clamp(36px,5.4vw,50px)] w-[clamp(36px,5.4vw,50px)]"
            >
              <rect
                x="5"
                y="8.5"
                width="11"
                height="11"
                rx="3"
                transform="rotate(45 10.5 14)"
                stroke="var(--cyan)"
              />
              <rect
                x="12"
                y="8.5"
                width="11"
                height="11"
                rx="3"
                transform="rotate(45 17.5 14)"
                stroke="var(--violet)"
              />
            </svg>
          </span>
          <span
            class="font-display text-[clamp(30px,5.2vw,46px)] leading-none font-semibold tracking-tight text-cyan-600 dark:text-cyan-400"
            >Exchange</span
          >
        </div>
        <span
          class="font-mono text-[clamp(10px,1.3vw,11.5px)] tracking-[0.26em] text-slate-400 uppercase dark:text-slate-500"
          >Le marché des joueurs</span
        >
      </div>

      <!-- Subtitle -->
      <p
        class="-mt-2 mb-4 max-w-[460px] text-base leading-relaxed text-slate-600 dark:text-slate-300"
      >
        Recherche une carte, colle une decklist, ou trouve un joueur pour parcourir sa collection.
        On te montre qui possède quoi, à quel prix, et tu composes l'échange.
      </p>

      <!-- Search area -->
      <div ref="searchAreaRef" class="flex w-full max-w-[540px] flex-col items-center gap-3.5">
        <SegToggle v-model="mode" :options="searchOptions" shortcuts />

        <!-- search-hero: glow via real child div + group-focus-within -->
        <div class="group relative w-full">
          <div
            :class="[
              'pointer-events-none absolute -inset-x-2.5 -inset-y-8 -z-10 rounded-[40px] opacity-50 blur-xl transition-opacity duration-300 group-focus-within:opacity-90',
              mode === 'player'
                ? 'bg-violet-500/20 dark:bg-violet-400/20'
                : 'bg-cyan-500/20 dark:bg-cyan-400/20',
            ]"
          />

          <!-- Name search -->
          <div
            v-if="mode === 'name'"
            class="flex min-h-[62px] items-center gap-2.5 rounded-2xl border border-solid border-slate-400/50 bg-slate-200/75 py-2 pr-2 pl-4 transition-all duration-200 focus-within:border-cyan-500/40 focus-within:ring-4 focus-within:ring-cyan-500/10 dark:border-white/15 dark:bg-black/20 dark:focus-within:border-cyan-400/40"
          >
            <Icon
              name="lucide:search"
              :size="16"
              class="shrink-0 text-slate-500/80 dark:text-slate-500"
            />
            <input
              v-model="q"
              class="min-w-0 flex-1 border-0 bg-transparent text-base text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
              placeholder="Vampiric Tutor, Sire of Seven Deaths…"
              @keydown.enter="navigateToSearch(q, mode)"
            />
            <button
              class="inline-flex items-center justify-center gap-2 self-stretch rounded-xl border border-transparent bg-cyan-500 px-6 text-base leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
              @click="navigateToSearch(q, mode)"
            >
              Chercher
            </button>
          </div>

          <!-- Player search -->
          <PlayerPicker v-else-if="mode === 'player'" v-model="player" @submit="goToPlayer" />

          <!-- Decklist search -->
          <div
            v-else
            class="flex flex-col items-start gap-3 rounded-2xl border border-slate-300 bg-black/20 py-2 pr-2 pl-4 transition-all duration-200 focus-within:border-cyan-500/40 focus-within:bg-black/10 focus-within:ring-4 focus-within:ring-cyan-500/10 dark:border-white/15 dark:focus-within:border-cyan-400/40"
          >
            <div class="flex w-full items-center gap-2.5">
              <Icon
                name="lucide:layers"
                size="20"
                class="flex-none text-slate-400 dark:text-slate-500"
              />
              <span class="text-sm text-slate-400 dark:text-slate-500"
                >Colle ta decklist (Moxfield · Archidekt · texte)</span
              >
            </div>
            <textarea
              v-model="decklistContent"
              :rows="4"
              class="w-full resize-y font-mono text-sm text-slate-800 dark:text-slate-100"
              placeholder="1x Vampiric Tutor&#10;1x Black Market Connections&#10;1x The Soul Stone…"
            />
            <button
              class="inline-flex items-center justify-center gap-2 self-end rounded-lg border border-transparent bg-cyan-500 px-3 py-1.5 text-xs leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
              @click="handleDecklistSearch"
            >
              Trouver les joueurs
            </button>
          </div>
        </div>

        <!-- Player mode helper -->
        <p
          v-if="mode === 'player' && !player"
          class="max-w-[420px] text-center text-xs text-slate-400 dark:text-slate-500"
        >
          Choisis un joueur : tu arrives sur sa collection, filtrable comme la tienne.
        </p>

        <!-- Recent searches -->
        <div
          v-if="mode === 'name' && recents.length > 0"
          class="flex flex-wrap items-center justify-center gap-2"
        >
          <span
            class="text-2xs mr-0.5 font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
            >récents</span
          >
          <button
            v-for="r in recents"
            :key="r"
            class="inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-slate-200 bg-slate-100 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-slate-600 transition-all duration-150 select-none hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
            @click="selectRecent(r)"
          >
            <Icon name="lucide:clock" size="12" />
            {{ r }}
          </button>
        </div>
      </div>
    </div>

    <!-- ── TRENDS ── -->
    <div class="mt-6">
      <div class="mb-3.5 flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <span class="text-cyan-600 dark:text-cyan-400"
            ><Icon name="lucide:trending-up" size="18"
          /></span>
          <h2 class="font-display m-0 text-base font-semibold tracking-tight">
            Tendances cette semaine
          </h2>
        </div>
        <a
          class="inline-flex items-center gap-1 text-sm text-slate-600 transition-colors duration-150 hover:text-cyan-600 dark:text-slate-300 dark:hover:text-cyan-400"
          href="#"
        >
          voir tout
          <Icon name="lucide:chevron-right" size="14" />
        </a>
      </div>

      <div
        class="grid [grid-template-columns:repeat(auto-fill,minmax(118px,1fr))] gap-4 max-md:[grid-template-columns:repeat(auto-fill,minmax(96px,1fr))] max-md:gap-3.5"
      >
        <CardCell
          v-for="t in trends"
          :key="t.name"
          :scryfall-id="t.id"
          :name="t.name"
          :price="t.price"
          @click="navigateTo('/search')"
        />
      </div>
    </div>

    <!-- ── SECONDARY DISCOVERY ── -->
    <div class="mt-7 flex flex-wrap gap-4">
      <!-- Collection panel — immersive tile, derived from the collection page's value bar -->
      <div
        class="relative min-h-[206px] min-w-[260px] flex-1 overflow-hidden rounded-2xl border border-slate-200 bg-white/60 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <!-- graph background -->
        <div class="absolute inset-0">
          <EnvelopeGraph v-if="hasEnoughCollectionHistory" :data="collectionEnvelopeData" />
          <div
            v-else
            class="text-2xs flex h-full items-center justify-center font-mono tracking-wide text-slate-400 uppercase dark:text-slate-500"
          >
            {{ collectionHistoryPending ? 'Chargement…' : "Pas encore assez d'historique" }}
          </div>
        </div>

        <!-- scrim -->
        <div
          class="pointer-events-none absolute inset-0 z-[1] bg-gradient-to-b from-white/95 via-white/55 to-transparent dark:from-zinc-900/95 dark:via-zinc-900/55"
        />
        <!-- bottom scrim — darkens the graph behind the meta line and CTA button -->
        <div
          class="pointer-events-none absolute inset-x-0 bottom-0 z-[2] h-24 bg-gradient-to-t from-white/95 to-transparent dark:from-zinc-900/95"
        />

        <!-- top: KPIs -->
        <div class="absolute inset-x-0 top-0 z-[3] flex flex-col gap-1.5 px-[18px] py-[18px]">
          <span
            class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
            >Ma collection · CardMarket</span
          >
          <span class="font-display text-[30px] leading-none font-semibold tracking-tight">{{
            formatPrice(collectionTotalValueCents)
          }}</span>
          <span
            :class="[
              'font-mono text-[12.5px]',
              collectionVariation.positive
                ? 'text-cyan-600 dark:text-cyan-400'
                : 'text-red-500 dark:text-red-400',
            ]"
            >{{ collectionVariation.positive ? '▴' : '▾' }}
            {{ formatPrice(Math.abs(collectionVariation.deltaCents)) }} ·
            {{ collectionVariation.positive ? '+' : '−'
            }}{{
              Math.abs(collectionVariation.pct).toLocaleString('fr-FR', {
                minimumFractionDigits: 1,
                maximumFractionDigits: 1,
              })
            }}
            % (30 j)</span
          >
        </div>

        <!-- meta: card counts -->
        <div
          v-if="collectionStatsData"
          class="absolute bottom-[62px] left-[18px] z-[3] flex items-center gap-2.5 font-mono text-[11.5px] whitespace-nowrap text-slate-500 [text-shadow:0_1px_4px_rgba(0,0,0,0.35)] dark:text-slate-300 dark:[text-shadow:0_1px_4px_rgba(0,0,0,0.6)]"
        >
          <span
            ><b class="font-semibold text-slate-700 dark:text-slate-100">{{
              collectionStatsData.total_cards.toLocaleString('fr-FR')
            }}</b>
            cartes</span
          >
          <i class="h-[3px] w-[3px] rounded-full bg-slate-400 dark:bg-slate-500" />
          <span
            ><b class="font-semibold text-slate-700 dark:text-slate-100">{{
              collectionStatsData.unique_cards
            }}</b>
            uniques</span
          >
        </div>

        <!-- CTA -->
        <button
          class="absolute right-[18px] bottom-4 left-[18px] z-[3] inline-flex items-center justify-center gap-2 rounded-xl border border-slate-200 bg-transparent px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
          @click="navigateTo('/collection')"
        >
          Ouvrir ma collection
          <Icon name="lucide:arrow-up-right" size="15" />
        </button>
      </div>

      <!-- Trades panel -->
      <div
        class="min-w-[260px] flex-1 rounded-2xl border border-slate-200 bg-white/60 p-5 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-2">
            <span
              class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
              >Échanges en cours</span
            >
            <span class="font-mono text-[11px] text-slate-400 dark:text-slate-500">{{
              activeTradesTotal
            }}</span>
          </div>
          <a
            class="inline-flex items-center gap-1 text-sm text-slate-600 transition-colors duration-150 hover:text-cyan-600 dark:text-slate-300 dark:hover:text-cyan-400"
            href="#"
            @click.prevent="navigateTo('/trade')"
          >
            voir tout
            <Icon name="lucide:chevron-right" size="13" />
          </a>
        </div>

        <div
          v-if="activeTradesPending && activeTrades.length === 0"
          class="flex items-center justify-center py-8 font-mono text-xs text-slate-400 dark:text-slate-500"
        >
          <Icon name="lucide:loader-circle" size="16" class="mr-2 animate-spin" />
          Chargement…
        </div>

        <p
          v-else-if="activeTrades.length === 0"
          class="mt-3 py-4 text-center font-mono text-xs text-slate-400 dark:text-slate-500"
        >
          Aucun échange en cours.
        </p>

        <div v-else class="mt-3 flex flex-col gap-2">
          <button
            v-for="t in activeTrades"
            :key="t.id"
            class="flex w-full items-center gap-3 rounded-xl border border-slate-200 bg-white px-3.5 py-3 text-left transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 dark:border-white/10 dark:bg-zinc-900 dark:hover:border-white/15 dark:hover:bg-zinc-800"
            @click="navigateTo(`/trade/${t.id}`)"
          >
            <PlayerAvatar :username="t.partner_username" />
            <span class="flex min-w-0 flex-1 flex-col">
              <span
                class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
                >{{ t.partner_username }}</span
              >
              <span class="text-xs text-slate-400 dark:text-slate-500"
                >{{ t.my_card_count }} donnée{{ t.my_card_count > 1 ? 's' : '' }} ·
                {{ t.partner_card_count }} reçue{{ t.partner_card_count > 1 ? 's' : '' }}</span
              >
            </span>
            <TradeStatusPill :status="toTradeStatus(t.status)" size="sm" />
            <Icon
              name="lucide:chevron-right"
              size="16"
              class="flex-none text-slate-400 dark:text-slate-500"
            />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
