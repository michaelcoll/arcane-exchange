<script setup lang="ts">
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeRating } from '~/utils/trade';

// Écran d'engagement : sur mobile, la barre d'action remplace la nav du bas (lu par app.vue).
definePageMeta({ middleware: 'auth', hideBottomNav: true });

const route = useRoute();
const tradeId = computed(() => route.params.id as string);

const { getTrade, removeCard, acceptTrade, abandonTrade, confirmTrade, rateTrade } =
  useTradeService();
const { showError } = useToast();

const { data: trade, pending, error, refresh } = await getTrade(tradeId);

const errorTitle = computed(() => {
  const code = (error.value as { statusCode?: number } | null)?.statusCode;
  if (code === 404) return "Cet échange n'existe pas";
  if (code === 403) return "Tu n'as pas accès à cet échange";
  return 'Impossible de charger cet échange';
});

const status = computed(() => toTradeStatus(trade.value?.status ?? 'PENDING'));
const partner = computed(() => trade.value?.partner_username ?? '');
const editable = computed(() => isTradeEditable(status.value));
const reserved = computed(() => isTradeReserved(status.value));

const giveTotal = computed(() => tradeCardsTotal(trade.value?.my_cards ?? []));
const getTotal = computed(() => tradeCardsTotal(trade.value?.partner_cards ?? []));
const diff = computed(() => getTotal.value - giveTotal.value);

const meAccepted = computed(() => trade.value?.me.accepted ?? false);
const meConfirmed = computed(() => trade.value?.me.confirmed ?? false);
const meRating = computed<TradeRating>(() => trade.value?.me.rating ?? null);
const partnerRating = computed<TradeRating>(() => trade.value?.partner.rating ?? null);

const busy = ref(false);

const run = async (action: () => Promise<unknown>) => {
  busy.value = true;
  try {
    await action();
  } catch (e) {
    const message = (e as { data?: { error?: string } })?.data?.error ?? 'Une erreur est survenue.';
    showError('Action refusée', message);
  } finally {
    await refresh();
    busy.value = false;
  }
};

type Modal = { kind: 'accept' } | { kind: 'abandon' } | { kind: 'modify'; run: () => void };
const modal = ref<Modal | null>(null);
const confirmation = computed(() => modal.value && tradeConfirmation(modal.value.kind, diff.value));

const confirmModal = () => {
  const current = modal.value;
  modal.value = null;
  if (current?.kind === 'accept') run(() => acceptTrade(tradeId.value));
  else if (current?.kind === 'abandon') run(() => abandonTrade(tradeId.value));
  else current?.run();
};

/* Mobile : menu « … » de la barre de page et bottom sheet des étapes. */
const menuOpen = ref(false);
const stepsOpen = ref(false);
const abandonable = computed(() => isTradeAbandonable(status.value));

const openSteps = () => {
  menuOpen.value = false;
  stepsOpen.value = true;
};
const askAbandon = () => {
  menuOpen.value = false;
  modal.value = { kind: 'abandon' };
};

const removeGet = (card: TradeCard) => {
  const doRemove = () =>
    run(() =>
      removeCard(tradeId.value, {
        set_code: card.set_code,
        collector_number: card.collector_number,
        language_code: card.language_code,
        foil: card.foil,
        owner_username: partner.value,
      }),
    );
  if (status.value === 'ONE_ACCEPTED') {
    modal.value = { kind: 'modify', run: doRemove };
    return;
  }
  doRemove();
};

const addGet = () => navigateTo(`/search?player=${encodeURIComponent(partner.value)}`);

const confirmExchange = () => run(() => confirmTrade(tradeId.value));
const rate = (value: number) => run(() => rateTrade(tradeId.value, value));

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

    <template v-else-if="trade">
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
            @{{ partner }}
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
                v-if="abandonable"
                role="menuitem"
                class="flex items-center gap-2.5 px-4 py-2.5 text-left text-sm text-red-600 hover:bg-red-500/10 dark:text-red-400"
                :disabled="busy"
                @click="askAbandon"
              >
                <Icon name="lucide:x" size="16" /> Abandonner l'échange
              </button>
            </div>
          </template>
        </div>

        <TradeStatusDots :status="status" @help="stepsOpen = true" />

        <div class="flex flex-col gap-[18px]">
          <TradeCardRail
            :owner="partner"
            side="receive"
            :cards="trade.partner_cards"
            :reserved="reserved"
            :removable="editable"
            empty-message="Tu n'as demandé aucune carte pour l'instant."
            :add-label="editable ? 'Chercher chez lui' : undefined"
            @remove="removeGet"
            @add="addGet"
          />

          <div class="flex items-center gap-2.5 px-0.5">
            <span class="h-px flex-1 bg-[var(--line-2)]" />
            <span
              :class="[
                'inline-flex items-center gap-1.5 rounded-full border px-[13px] py-1.5 font-mono text-[11.5px] font-semibold whitespace-nowrap',
                reserved
                  ? 'border-secondary/40 bg-secondary/15 text-secondary-ink'
                  : 'border-[var(--line-2)] bg-slate-400/10 text-[var(--ink-2)]',
              ]"
            >
              <Icon :name="reserved ? 'lucide:lock' : 'lucide:arrow-left-right'" size="12" />
              {{ reserved ? 'échange réservé' : 'échange' }}
            </span>
            <span class="h-px flex-1 bg-[var(--line-2)]" />
          </div>

          <TradeCardRail
            :owner="null"
            side="give"
            :cards="trade.my_cards"
            :reserved="reserved"
            :removable="false"
            :empty-message="`@${partner} n'a demandé aucune de tes cartes.`"
          />
        </div>

        <TradeRatingSection
          v-if="status === 'COMPLETED' || status === 'CLOSED'"
          :partner="partner"
          :me-rating="meRating"
          :partner-rating="partnerRating"
          :busy="busy"
          @rate="rate"
        />

        <TradeActionBar
          :status="status"
          :me-accepted="meAccepted"
          :me-confirmed="meConfirmed"
          :partner="partner"
          :diff="diff"
          :busy="busy"
          @accept="modal = { kind: 'accept' }"
          @confirm="confirmExchange"
        />

        <TradeStepsSheet
          v-if="stepsOpen"
          :status="status"
          :partner="partner"
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
              <PlayerAvatar :username="partner" />
              <h2 class="font-display text-base font-semibold tracking-tight">
                Échange avec
                <span class="text-secondary">{{ partner }}</span>
              </h2>
            </div>
          </div>
          <TradeStatusPill :status="status" />
        </div>

        <div :class="[panel, 'mb-4 px-4 pt-5 pb-4']">
          <div
            v-if="status === 'ABANDONED'"
            class="flex items-center justify-center gap-2.5 text-red-600 dark:text-red-400"
          >
            <Icon name="lucide:x" size="18" />
            <span class="font-semibold">Transaction abandonnée — cartes libérées</span>
          </div>
          <TradeLifecycle v-else :status="status" />
        </div>

        <TradeStatusBanner
          class="mb-4"
          :status="status"
          :counterparty="partner"
          :accepted="meAccepted"
          :confirmed="meConfirmed"
        />

        <div
          class="grid [grid-template-columns:1fr_auto_1fr] items-stretch gap-4 max-md:[grid-template-columns:1fr]"
        >
          <div class="flex flex-col gap-2">
            <TradeColumn
              label="Je donne"
              :cards="trade.my_cards"
              tone="neutral"
              :reserved="reserved"
              :removable="false"
            />
            <p :class="hint">
              Cartes demandées par <span class="text-secondary">{{ partner }}</span> · non
              retirables depuis cet écran.
            </p>
          </div>

          <div class="flex min-w-[168px] flex-col items-center justify-center gap-3.5">
            <TradeBalance :diff="diff" :give-total="giveTotal" :get-total="getTotal" />
            <span
              class="text-2xs inline-flex items-center gap-1.5 font-mono tracking-wide text-slate-400 dark:text-slate-500"
            >
              <Icon name="lucide:info" size="13" /> Réglé hors plateforme
            </span>
          </div>

          <TradeColumn
            label="Je reçois"
            :cards="trade.partner_cards"
            tone="secondary"
            :reserved="reserved"
            :removable="editable"
            add-label="Chercher dans sa collection"
            @remove="removeGet"
            @add="addGet"
          />
        </div>

        <!-- ACTIONS SELON LE STATUT -->
        <div :class="[panel, 'mt-[18px] p-4']">
          <div
            v-if="status === 'PENDING'"
            class="flex flex-wrap items-center justify-between gap-3.5"
          >
            <span :class="hint">Modifiable par les deux parties · aucune carte réservée</span>
            <div class="flex flex-wrap items-center gap-2.5">
              <button :class="btnDanger" :disabled="busy" @click="modal = { kind: 'abandon' }">
                Abandonner
              </button>
              <button :class="btnPrimary" :disabled="busy" @click="modal = { kind: 'accept' }">
                <Icon name="lucide:check" size="16" /> Accepter l’échange
              </button>
            </div>
          </div>

          <div
            v-else-if="status === 'ONE_ACCEPTED'"
            class="flex flex-wrap items-center justify-between gap-3.5"
          >
            <span :class="[hint, 'inline-flex items-center gap-1.5']">
              <Icon name="lucide:lock" size="12" class="text-secondary" />
              Cartes réservées · modifiable (repasse en négociation)
            </span>
            <div class="flex flex-wrap items-center gap-2.5">
              <button :class="btnDanger" :disabled="busy" @click="modal = { kind: 'abandon' }">
                Abandonner
              </button>
              <button
                v-if="!meAccepted"
                :class="btnPrimary"
                :disabled="busy"
                @click="modal = { kind: 'accept' }"
              >
                <Icon name="lucide:check" size="16" /> Accepter à mon tour
              </button>
            </div>
          </div>

          <div
            v-else-if="status === 'FULLY_ACCEPTED'"
            class="flex flex-wrap items-center justify-between gap-3.5"
          >
            <span :class="hint"
              >Échange physique en personne · confirmez chacun une fois réalisé</span
            >
            <div class="flex flex-wrap items-center gap-2.5">
              <button :class="btnDanger" :disabled="busy" @click="modal = { kind: 'abandon' }">
                Abandonner
              </button>
              <span
                v-if="meConfirmed"
                class="border-secondary/30 bg-secondary/10 text-secondary-ink inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap"
              >
                <Icon name="lucide:check" size="13" /> Tu as confirmé · en attente de
                <span class="text-secondary">{{ partner }}</span>
              </span>
              <button v-else :class="btnPrimary" :disabled="busy" @click="confirmExchange">
                <Icon name="lucide:check" size="16" /> Confirmer « échange réalisé »
              </button>
            </div>
          </div>

          <div v-else-if="status === 'COMPLETED'" class="flex flex-col gap-3.5">
            <div class="flex flex-wrap items-center justify-between gap-3.5">
              <div class="flex flex-col gap-0.5">
                <span :class="label"
                  >Noter <span class="text-secondary">{{ partner }}</span></span
                >
                <span :class="hint">Optionnel · 1 à 5 étoiles</span>
              </div>
              <div v-if="meRating == null" class="flex flex-wrap items-center gap-3.5">
                <TradeRatingStars :value="null" @rate="rate" />
                <button
                  class="text-xs font-medium text-slate-500 underline-offset-2 hover:underline disabled:opacity-50 dark:text-slate-400"
                  :disabled="busy"
                  @click="rate(0)"
                >
                  Passer la notation
                </button>
              </div>
              <span
                v-else
                class="border-secondary/30 bg-secondary/10 text-secondary-ink inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap"
              >
                <Icon :name="meRating === 0 ? 'lucide:circle-minus' : 'mdi:star'" size="13" />
                {{ meRating === 0 ? 'Notation passée' : `Tu as mis ${meRating}/5` }}
              </span>
            </div>
            <div class="h-px bg-slate-200 dark:bg-white/10" />
            <div class="flex flex-wrap items-center justify-between gap-3">
              <span v-if="partnerRating != null" :class="hint">
                <span class="text-secondary">{{ partner }}</span>
                {{ partnerRating === 0 ? 'a passé la notation.' : 'a noté de son côté.' }}
              </span>
              <span v-else :class="hint">
                En attente éventuelle de la note de
                <span class="text-secondary">{{ partner }}</span
                >.
              </span>
            </div>
          </div>

          <div
            v-else-if="status === 'CLOSED'"
            class="flex flex-wrap items-center gap-x-[18px] gap-y-2"
          >
            <span :class="hint"
              >Ta note :
              <span class="text-secondary font-semibold">{{
                formatTradeRating(meRating)
              }}</span></span
            >
            <span :class="hint"
              >Note de <span class="text-secondary">{{ partner }}</span> :
              <span class="text-secondary font-semibold">{{
                formatTradeRating(partnerRating)
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
        v-if="confirmation"
        :title="confirmation.title"
        :body="confirmation.body"
        :confirm-label="confirmation.confirmLabel"
        :tone="confirmation.tone"
        @cancel="modal = null"
        @confirm="confirmModal"
      />
    </template>
  </div>
</template>
