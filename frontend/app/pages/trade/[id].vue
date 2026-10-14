<script setup lang="ts">
import type { TradeCard } from '~/bindings/TradeCard';

// Écran d'engagement : sur mobile, la barre d'action remplace la nav du bas (lu par app.vue).
definePageMeta({ middleware: 'auth', hideBottomNav: true });

const route = useRoute();
const tradeId = computed(() => route.params.id as string);

const { getTrade, removeCard, acceptTrade, abandonTrade, confirmTrade, rateTrade } =
  useTradeService();
const { showError } = useToast();

const {
  data: trade,
  pending,
  error,
  refresh,
} = await useAsyncData(`trade-${tradeId.value}`, () => getTrade(tradeId.value), { lazy: true });

const errorTitle = computed(() => {
  const { status } = readApiError(error.value);
  if (status === 404) return "Cet échange n'existe pas";
  if (status === 403) return "Tu n'as pas accès à cet échange";
  return 'Impossible de charger cet échange';
});

/* La vue du Trade côté joueur décide de tout ce que l'écran dit et permet ; la page ne fait que
 * la câbler aux services et l'afficher. */
const {
  view,
  requestAccept,
  requestAbandon,
  confirmPending,
  cancelPending,
  removeReceivedCard,
  confirmExchange,
  rate,
  skipRating,
} = useTradeView(trade, {
  accept: () => acceptTrade(tradeId.value),
  abandon: () => abandonTrade(tradeId.value),
  confirmExchange: () => confirmTrade(tradeId.value),
  rate: (rating) => rateTrade(tradeId.value, rating),
  removeCard: (card) =>
    removeCard(tradeId.value, {
      set_code: card.set_code,
      collector_number: card.collector_number,
      language_code: card.language_code,
      foil: card.foil,
      owner_username: view.value?.partnerUsername ?? '',
    }),
  reload: () => refresh(),
  reportRefusal: (message) => showError('Action refusée', message),
});

/* The page bar's "…" menu (mobile) and the steps sheet (both regions). */
const menuOpen = ref(false);
const stepsOpen = ref(false);

/* The card whose detail modal is open, from either side and either region. */
const detail = ref<TradeCard | null>(null);

const openSteps = () => {
  menuOpen.value = false;
  stepsOpen.value = true;
};
const askAbandon = () => {
  menuOpen.value = false;
  requestAbandon();
};

const addGet = () =>
  navigateTo(searchRoute({ mode: 'player', player: view.value?.partnerUsername ?? '' }));
</script>

<template>
  <div
    class="mx-auto max-w-[1180px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-3 max-md:pb-[calc(2rem+env(safe-area-inset-bottom))]"
  >
    <div
      v-if="pending && !trade"
      class="flex items-center justify-center py-20 font-mono text-sm text-slate-400 dark:text-slate-500"
    >
      <Icon name="lucide:loader-circle" size="18" class="mr-2.5 animate-spin" />
      Chargement…
    </div>

    <div
      v-else-if="error"
      class="flex flex-col items-center justify-center gap-4 py-20 text-slate-400 dark:text-slate-500"
    >
      <Icon name="lucide:alert-triangle" :size="48" class="opacity-40" />
      <p class="text-center font-mono text-base">{{ errorTitle }}</p>
      <NuxtLink
        to="/trade"
        class="bg-primary hover:bg-primary-soft inline-flex items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-[var(--on-primary)] shadow-lg transition-all duration-150 hover:-translate-y-px active:translate-y-0"
        >Retour aux échanges</NuxtLink
      >
    </div>

    <template v-else-if="view">
      <!-- MOBILE (< md) : design de l'écran de trade iOS -->
      <div data-part="mobile" class="flex flex-col gap-5 md:hidden">
        <div class="relative grid [grid-template-columns:2.25rem_1fr_2.25rem] items-center gap-2">
          <button
            class="grid h-9 w-9 place-items-center rounded-lg text-[var(--ink-2)] transition-colors duration-150 hover:bg-slate-400/10 hover:text-[var(--ink)]"
            aria-label="Retour"
            @click="$router.back()"
          >
            <Icon name="lucide:chevron-left" size="20" />
          </button>
          <h2 class="font-display text-secondary truncate text-center text-base font-semibold">
            @{{ view.partnerUsername }}
          </h2>
          <button
            class="grid h-9 w-9 place-items-center rounded-lg text-[var(--ink-2)] transition-colors duration-150 hover:bg-slate-400/10 hover:text-[var(--ink)]"
            aria-label="Options"
            aria-haspopup="menu"
            :aria-expanded="menuOpen"
            @click="menuOpen = !menuOpen"
          >
            <Icon name="lucide:ellipsis" size="20" />
          </button>

          <template v-if="menuOpen">
            <div class="fixed inset-0 z-[60]" @click="menuOpen = false" />
            <div
              role="menu"
              class="absolute top-full right-0 z-[61] mt-1.5 flex min-w-[240px] animate-[pop_0.2s_cubic-bezier(0.3,1.2,0.4,1)] flex-col overflow-hidden rounded-xl border border-slate-300 bg-white py-1 shadow-2xl dark:border-white/15 dark:bg-zinc-900"
              @keydown.esc="menuOpen = false"
            >
              <button
                role="menuitem"
                class="flex items-center gap-2.5 px-4 py-2.5 text-left text-sm hover:bg-slate-400/10"
                @click="openSteps"
              >
                <Icon name="lucide:info" size="16" /> Les étapes d'un échange
              </button>
              <button
                v-if="view.abandonable"
                role="menuitem"
                class="flex items-center gap-2.5 px-4 py-2.5 text-left text-sm text-red-600 hover:bg-red-500/10 dark:text-red-400"
                :disabled="view.busy"
                @click="askAbandon"
              >
                <Icon name="lucide:x" size="16" /> Abandonner l'échange
              </button>
            </div>
          </template>
        </div>

        <TradeStatusDots
          :presentation="view.presentation"
          :steps="view.steps"
          @help="stepsOpen = true"
        />

        <div class="flex flex-col gap-[18px]">
          <TradeCardRail
            :owner="view.partnerUsername"
            direction="receive"
            :side="view.receive"
            :reserved="view.reserved"
            :removable="view.editable"
            :busy="view.busy"
            empty-message="Tu n'as demandé aucune carte pour l'instant."
            :add-label="view.editable ? 'Chercher chez lui' : undefined"
            @remove="removeReceivedCard"
            @add="addGet"
            @open="detail = $event"
          />

          <TradeSideSeparator :reserved="view.reserved" />

          <TradeCardRail
            :owner="null"
            direction="give"
            :side="view.give"
            :reserved="view.reserved"
            :removable="false"
            :empty-message="`@${view.partnerUsername} n'a demandé aucune de tes cartes.`"
            @open="detail = $event"
          />
        </div>

        <TradeRatingSection
          v-if="view.rating.visible"
          :partner="view.partnerUsername"
          :rating="view.rating"
          :busy="view.busy"
          @rate="rate"
          @skip="skipRating"
        />

        <TradeActionBar
          :action="view.primaryAction"
          :partner="view.partnerUsername"
          :busy="view.busy"
          @accept="requestAccept"
          @confirm="confirmExchange"
        />
      </div>

      <!-- DESKTOP (≥ md) -->
      <div data-part="desktop" class="flex flex-col gap-4 max-md:hidden">
        <div class="flex flex-wrap items-center justify-between gap-3.5">
          <div class="flex min-w-0 items-center gap-3">
            <button
              class="grid h-9 w-9 flex-none place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
              aria-label="Retour"
              @click="$router.back()"
            >
              <Icon name="lucide:chevron-left" size="16" />
            </button>
            <PlayerAvatar :username="view.partnerUsername" />
            <h2 class="font-display text-secondary truncate text-base font-semibold tracking-tight">
              @{{ view.partnerUsername }}
            </h2>
          </div>
          <TradeStatusDots
            :presentation="view.presentation"
            :steps="view.steps"
            @help="stepsOpen = true"
          />
        </div>

        <!-- The middle column is as wide as the "échange réservé" separator, so the sides keep
             the same width whether the cards are reserved or not. -->
        <div
          class="grid [grid-template-columns:minmax(0,1fr)_160px_minmax(0,1fr)] items-stretch gap-5"
        >
          <TradeCardRail
            layout="grid"
            :owner="view.partnerUsername"
            direction="receive"
            :side="view.receive"
            :reserved="view.reserved"
            :removable="view.editable"
            :busy="view.busy"
            empty-message="Tu n'as demandé aucune carte pour l'instant."
            :add-label="view.editable ? 'Chercher chez lui' : undefined"
            @remove="removeReceivedCard"
            @add="addGet"
            @open="detail = $event"
          />

          <TradeSideSeparator orientation="vertical" :reserved="view.reserved" />

          <TradeCardRail
            layout="grid"
            :owner="null"
            direction="give"
            :side="view.give"
            :reserved="view.reserved"
            :removable="false"
            :empty-message="`@${view.partnerUsername} n'a demandé aucune de tes cartes.`"
            @open="detail = $event"
          />
        </div>

        <TradeActionPanel
          :view="view"
          @accept="requestAccept"
          @confirm="confirmExchange"
          @abandon="requestAbandon"
          @rate="rate"
          @skip="skipRating"
        />
      </div>

      <!-- Shared by both regions: opened by the step dots or by the mobile "…" menu. -->
      <TradeStepsSheet
        v-if="stepsOpen"
        :steps="view.steps"
        :partner="view.partnerUsername"
        @close="stepsOpen = false"
      />

      <CardDetailModal v-if="detail" :card="detail" @close="detail = null" />

      <TradeConfirmModal
        v-if="view.confirmation"
        :title="view.confirmation.title"
        :body="view.confirmation.body"
        :confirm-label="view.confirmation.confirmLabel"
        :tone="view.confirmation.tone"
        @cancel="cancelPending"
        @confirm="confirmPending"
      />
    </template>
  </div>
</template>
