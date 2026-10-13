<script setup lang="ts">
// Écran d'engagement : sur mobile, la barre d'action remplace la nav du bas (lu par app.vue).
definePageMeta({ middleware: 'auth', hideBottomNav: true });

const route = useRoute();
const tradeId = computed(() => route.params.id as string);

const { getTrade, removeCard, acceptTrade, abandonTrade, confirmTrade, rateTrade } =
  useTradeService();
const { showError } = useToast();

const { data: trade, pending, error, refresh } = await getTrade(tradeId);

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

/* Mobile : menu « … » de la barre de page et bottom sheet des étapes. */
const menuOpen = ref(false);
const stepsOpen = ref(false);

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

const btnBase =
  'inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap transition-all duration-150 hover:-translate-y-px active:translate-y-0 disabled:pointer-events-none disabled:opacity-50';
const btnDanger = `${btnBase} border border-red-500/40 bg-transparent text-red-600 hover:bg-red-500/10 dark:border-red-400/40 dark:text-red-400 dark:hover:bg-red-400/10`;
const btnPrimary = `${btnBase} border border-transparent bg-primary font-bold text-[var(--on-primary)] shadow-lg hover:bg-primary-soft`;

const panel =
  'rounded-2xl border border-slate-200 bg-white/60 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60';
const hint = 'text-xs text-slate-400 dark:text-slate-500';
const label =
  'text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500';
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
      <div class="flex flex-col gap-5 md:hidden">
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

        <TradeStatusDots :status="view.status" @help="stepsOpen = true" />

        <div class="flex flex-col gap-[18px]">
          <TradeCardRail
            :owner="view.partnerUsername"
            side="receive"
            :cards="view.receive.cards"
            :reserved="view.reserved"
            :removable="view.editable"
            empty-message="Tu n'as demandé aucune carte pour l'instant."
            :add-label="view.editable ? 'Chercher chez lui' : undefined"
            @remove="removeReceivedCard"
            @add="addGet"
          />

          <div class="flex items-center gap-2.5 px-0.5">
            <span class="h-px flex-1 bg-[var(--line-2)]" />
            <span
              :class="[
                'inline-flex items-center gap-1.5 rounded-full border px-[13px] py-1.5 font-mono text-[11.5px] font-semibold whitespace-nowrap',
                view.reserved
                  ? 'border-secondary/40 bg-secondary/15 text-secondary-ink'
                  : 'border-[var(--line-2)] bg-slate-400/10 text-[var(--ink-2)]',
              ]"
            >
              <Icon :name="view.reserved ? 'lucide:lock' : 'lucide:arrow-left-right'" size="12" />
              {{ view.reserved ? 'échange réservé' : 'échange' }}
            </span>
            <span class="h-px flex-1 bg-[var(--line-2)]" />
          </div>

          <TradeCardRail
            :owner="null"
            side="give"
            :cards="view.give.cards"
            :reserved="view.reserved"
            :removable="false"
            :empty-message="`@${view.partnerUsername} n'a demandé aucune de tes cartes.`"
          />
        </div>

        <TradeRatingSection
          v-if="view.rating.visible"
          :partner="view.partnerUsername"
          :me-rating="view.rating.mine"
          :partner-rating="view.rating.partner"
          :busy="view.busy"
          @rate="rate"
        />

        <TradeActionBar
          :status="view.status"
          :me-accepted="view.me.accepted"
          :me-confirmed="view.me.confirmed"
          :partner="view.partnerUsername"
          :diff="view.balance.delta"
          :busy="view.busy"
          @accept="requestAccept"
          @confirm="confirmExchange"
        />

        <TradeStepsSheet
          v-if="stepsOpen"
          :status="view.status"
          :partner="view.partnerUsername"
          @close="stepsOpen = false"
        />
      </div>

      <!-- DESKTOP (≥ md) -->
      <div class="max-md:hidden">
        <div class="mb-4 flex flex-wrap items-center justify-between gap-3.5">
          <div class="flex items-center gap-3">
            <button
              class="grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
              aria-label="Retour"
              @click="$router.back()"
            >
              <Icon name="lucide:chevron-left" size="16" />
            </button>
            <div class="flex items-center gap-2.5">
              <PlayerAvatar :username="view.partnerUsername" />
              <h2 class="font-display text-base font-semibold tracking-tight">
                Échange avec
                <span class="text-secondary">{{ view.partnerUsername }}</span>
              </h2>
            </div>
          </div>
          <TradeStatusPill :status="view.status" />
        </div>

        <div :class="[panel, 'mb-4 px-4 pt-5 pb-4']">
          <div
            v-if="view.status === 'ABANDONED'"
            class="flex items-center justify-center gap-2.5 text-red-600 dark:text-red-400"
          >
            <Icon name="lucide:x" size="18" />
            <span class="font-semibold">Transaction abandonnée — cartes libérées</span>
          </div>
          <TradeLifecycle v-else :status="view.status" />
        </div>

        <TradeStatusBanner
          class="mb-4"
          :status="view.status"
          :counterparty="view.partnerUsername"
          :accepted="view.me.accepted"
          :confirmed="view.me.confirmed"
        />

        <div
          class="grid [grid-template-columns:1fr_auto_1fr] items-stretch gap-4 max-md:[grid-template-columns:1fr]"
        >
          <div class="flex flex-col gap-2">
            <TradeColumn
              label="Je donne"
              :cards="view.give.cards"
              tone="neutral"
              :reserved="view.reserved"
              :removable="false"
            />
            <p :class="hint">
              Cartes demandées par <span class="text-secondary">{{ view.partnerUsername }}</span> ·
              non retirables depuis cet écran.
            </p>
          </div>

          <div class="flex min-w-[168px] flex-col items-center justify-center gap-3.5">
            <TradeBalance
              :diff="view.balance.delta"
              :give-total="view.give.total"
              :get-total="view.receive.total"
            />
            <span
              class="text-2xs inline-flex items-center gap-1.5 font-mono tracking-wide text-slate-400 dark:text-slate-500"
            >
              <Icon name="lucide:info" size="13" /> Réglé hors plateforme
            </span>
          </div>

          <TradeColumn
            label="Je reçois"
            :cards="view.receive.cards"
            tone="secondary"
            :reserved="view.reserved"
            :removable="view.editable"
            add-label="Chercher dans sa collection"
            @remove="removeReceivedCard"
            @add="addGet"
          />
        </div>

        <!-- ACTIONS SELON LE STATUT -->
        <div :class="[panel, 'mt-[18px] p-4']">
          <div
            v-if="view.status === 'PENDING'"
            class="flex flex-wrap items-center justify-between gap-3.5"
          >
            <span :class="hint">Modifiable par les deux parties · aucune carte réservée</span>
            <div class="flex flex-wrap items-center gap-2.5">
              <button :class="btnDanger" :disabled="view.busy" @click="requestAbandon">
                Abandonner
              </button>
              <button :class="btnPrimary" :disabled="view.busy" @click="requestAccept">
                <Icon name="lucide:check" size="16" /> Accepter l’échange
              </button>
            </div>
          </div>

          <div
            v-else-if="view.status === 'ONE_ACCEPTED'"
            class="flex flex-wrap items-center justify-between gap-3.5"
          >
            <span :class="[hint, 'inline-flex items-center gap-1.5']">
              <Icon name="lucide:lock" size="12" class="text-secondary" />
              Cartes réservées · modifiable (repasse en négociation)
            </span>
            <div class="flex flex-wrap items-center gap-2.5">
              <button :class="btnDanger" :disabled="view.busy" @click="requestAbandon">
                Abandonner
              </button>
              <button
                v-if="!view.me.accepted"
                :class="btnPrimary"
                :disabled="view.busy"
                @click="requestAccept"
              >
                <Icon name="lucide:check" size="16" /> Accepter à mon tour
              </button>
            </div>
          </div>

          <div
            v-else-if="view.status === 'FULLY_ACCEPTED'"
            class="flex flex-wrap items-center justify-between gap-3.5"
          >
            <span :class="hint"
              >Échange physique en personne · confirmez chacun une fois réalisé</span
            >
            <div class="flex flex-wrap items-center gap-2.5">
              <button :class="btnDanger" :disabled="view.busy" @click="requestAbandon">
                Abandonner
              </button>
              <span
                v-if="view.me.confirmed"
                class="border-secondary/30 bg-secondary/10 text-secondary-ink inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap"
              >
                <Icon name="lucide:check" size="13" /> Tu as confirmé · en attente de
                <span class="text-secondary">{{ view.partnerUsername }}</span>
              </span>
              <button v-else :class="btnPrimary" :disabled="view.busy" @click="confirmExchange">
                <Icon name="lucide:check" size="16" /> Confirmer « échange réalisé »
              </button>
            </div>
          </div>

          <div v-else-if="view.status === 'COMPLETED'" class="flex flex-col gap-3.5">
            <div class="flex flex-wrap items-center justify-between gap-3.5">
              <div class="flex flex-col gap-0.5">
                <span :class="label"
                  >Noter <span class="text-secondary">{{ view.partnerUsername }}</span></span
                >
                <span :class="hint">Optionnel · 1 à 5 étoiles</span>
              </div>
              <div v-if="view.rating.canRate" class="flex flex-wrap items-center gap-3.5">
                <TradeRatingStars :value="null" @rate="rate" />
                <button
                  class="text-xs font-medium text-slate-500 underline-offset-2 hover:underline disabled:opacity-50 dark:text-slate-400"
                  :disabled="view.busy"
                  @click="skipRating"
                >
                  Passer la notation
                </button>
              </div>
              <span
                v-else
                class="border-secondary/30 bg-secondary/10 text-secondary-ink inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap"
              >
                <Icon
                  :name="view.rating.mine === 0 ? 'lucide:circle-minus' : 'mdi:star'"
                  size="13"
                />
                {{ view.rating.mine === 0 ? 'Notation passée' : `Tu as mis ${view.rating.mine}/5` }}
              </span>
            </div>
            <div class="h-px bg-slate-200 dark:bg-white/10" />
            <div class="flex flex-wrap items-center justify-between gap-3">
              <span v-if="view.rating.partner != null" :class="hint">
                <span class="text-secondary">{{ view.partnerUsername }}</span>
                {{ view.rating.partner === 0 ? 'a passé la notation.' : 'a noté de son côté.' }}
              </span>
              <span v-else :class="hint">
                En attente éventuelle de la note de
                <span class="text-secondary">{{ view.partnerUsername }}</span
                >.
              </span>
            </div>
          </div>

          <div
            v-else-if="view.status === 'CLOSED'"
            class="flex flex-wrap items-center gap-x-[18px] gap-y-2"
          >
            <span :class="hint"
              >Ta note :
              <span class="text-secondary font-semibold">{{
                formatTradeRating(view.rating.mine)
              }}</span></span
            >
            <span :class="hint"
              >Note de <span class="text-secondary">{{ view.partnerUsername }}</span> :
              <span class="text-secondary font-semibold">{{
                formatTradeRating(view.rating.partner)
              }}</span></span
            >
          </div>

          <!-- ABANDONED -->
          <div v-else class="flex flex-wrap items-center justify-between gap-3.5">
            <span :class="hint">Cette transaction est close.</span>
          </div>
        </div>
      </div>

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
