<script setup lang="ts">
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeRating } from '~/utils/trade';

definePageMeta({ middleware: 'auth' });

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

/* ---------- dérivés (aucun recalcul, tout vient du backend) ---------- */
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

/* ---------- actions ---------- */
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

const formatRating = (r: TradeRating) => (r == null ? 'non notée' : `${r}/5`);

/* ---------- styles de boutons partagés ---------- */
const btnBase =
  'inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap transition-all duration-150 hover:-translate-y-px active:translate-y-0 disabled:pointer-events-none disabled:opacity-50';
const btnGhost = `${btnBase} border border-slate-200 bg-transparent text-slate-600 hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100`;
const btnDanger = `${btnBase} border border-red-500/40 bg-transparent text-red-600 hover:bg-red-500/10 dark:border-red-400/40 dark:text-red-400 dark:hover:bg-red-400/10`;
const btnPrimary = `${btnBase} border border-transparent bg-cyan-500 font-bold text-zinc-950 shadow-lg hover:bg-cyan-400 dark:bg-cyan-400 dark:hover:bg-cyan-300`;

const panel =
  'rounded-2xl border border-slate-200 bg-white/60 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60';
const hint = 'text-xs text-slate-400 dark:text-slate-500';
const label =
  'text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500';
</script>

<template>
  <div class="mx-auto max-w-[1180px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <!-- LOADING -->
    <div
      v-if="pending && !trade"
      class="flex items-center justify-center py-20 font-mono text-sm text-slate-400 dark:text-slate-500"
    >
      <Icon name="lucide:loader-circle" size="18" class="mr-2.5 animate-spin" />
      Chargement…
    </div>

    <!-- ERROR -->
    <div
      v-else-if="error"
      class="flex flex-col items-center justify-center gap-4 py-20 text-slate-400 dark:text-slate-500"
    >
      <Icon name="lucide:alert-triangle" :size="48" class="opacity-40" />
      <p class="text-center font-mono text-base">{{ errorTitle }}</p>
      <NuxtLink
        to="/trade"
        class="inline-flex items-center justify-center gap-2 rounded-xl border border-transparent bg-cyan-500 px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
        >Retour aux échanges</NuxtLink
      >
    </div>

    <!-- CONTENT -->
    <template v-else-if="trade">
      <!-- HEADER -->
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
            <PlayerAvatar :initials="partner.slice(0, 2).toUpperCase()" />
            <h2 class="font-display text-base font-semibold tracking-tight">
              Échange avec
              <span class="text-cyan-600 dark:text-cyan-400">{{ partner }}</span>
            </h2>
          </div>
        </div>
        <TradeStatusPill :status="status" />
      </div>

      <!-- CYCLE DE VIE -->
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

      <!-- BANNIÈRE CONTEXTUELLE -->
      <TradeStatusBanner
        class="mb-4"
        :status="status"
        :counterparty="partner"
        :accepted="meAccepted"
        :confirmed="meConfirmed"
      />

      <!-- COLONNES + BALANCE -->
      <div
        class="grid [grid-template-columns:1fr_auto_1fr] items-stretch gap-4 max-md:[grid-template-columns:1fr]"
      >
        <div class="flex flex-col gap-2">
          <TradeColumn
            label="Je donne"
            :cards="trade.my_cards"
            accent="neutral"
            :reserved="reserved"
            :removable="false"
          />
          <p :class="hint">Cartes demandées par {{ partner }} · non retirables depuis cet écran.</p>
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
          accent="cyan"
          :reserved="reserved"
          :removable="editable"
          add-label="Chercher dans sa collection"
          @remove="removeGet"
          @add="addGet"
        />
      </div>

      <!-- ACTIONS SELON LE STATUT -->
      <div :class="[panel, 'mt-[18px] p-4']">
        <!-- PENDING -->
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

        <!-- ONE_ACCEPTED -->
        <div
          v-else-if="status === 'ONE_ACCEPTED'"
          class="flex flex-wrap items-center justify-between gap-3.5"
        >
          <span :class="[hint, 'inline-flex items-center gap-1.5']">
            <Icon name="lucide:lock" size="12" class="text-violet-500 dark:text-violet-300" />
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

        <!-- FULLY_ACCEPTED -->
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
              class="inline-flex items-center gap-1.5 rounded-full border border-violet-500/30 bg-violet-500/10 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-violet-700 dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300"
            >
              <Icon name="lucide:check" size="13" /> Tu as confirmé · en attente de {{ partner }}
            </span>
            <button v-else :class="btnPrimary" :disabled="busy" @click="confirmExchange">
              <Icon name="lucide:check" size="16" /> Confirmer « échange réalisé »
            </button>
          </div>
        </div>

        <!-- COMPLETED -->
        <div v-else-if="status === 'COMPLETED'" class="flex flex-col gap-3.5">
          <div class="flex flex-wrap items-center justify-between gap-3.5">
            <div class="flex flex-col gap-0.5">
              <span :class="label">Noter {{ partner }}</span>
              <span :class="hint">Optionnel · 0 à 5 étoiles</span>
            </div>
            <TradeRatingStars v-if="meRating == null" :value="null" @rate="rate" />
            <span
              v-else
              class="inline-flex items-center gap-1.5 rounded-full border border-violet-500/30 bg-violet-500/10 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-violet-700 dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300"
            >
              <Icon name="mdi:star" size="13" /> Tu as mis {{ meRating }}/5
            </span>
          </div>
          <div class="h-px bg-slate-200 dark:bg-white/10" />
          <div class="flex flex-wrap items-center justify-between gap-3">
            <span :class="hint">
              {{
                partnerRating != null
                  ? `${partner} a noté de son côté.`
                  : `En attente éventuelle de la note de ${partner}.`
              }}
            </span>
          </div>
        </div>

        <!-- CLOSED -->
        <div
          v-else-if="status === 'CLOSED'"
          class="flex flex-wrap items-center gap-x-[18px] gap-y-2"
        >
          <span :class="hint"
            >Ta note :
            <span class="font-semibold text-violet-500 dark:text-violet-300">{{
              formatRating(meRating)
            }}</span></span
          >
          <span :class="hint"
            >Note de {{ partner }} :
            <span class="font-semibold text-violet-500 dark:text-violet-300">{{
              formatRating(partnerRating)
            }}</span></span
          >
        </div>

        <!-- ABANDONED -->
        <div v-else class="flex flex-wrap items-center justify-between gap-3.5">
          <span :class="hint">Cette transaction est close.</span>
        </div>
      </div>

      <!-- MODALES -->
      <TradeConfirmModal
        v-if="modal?.kind === 'accept'"
        title="Accepter cet échange ?"
        body="Une fois acceptée, la transaction sera verrouillée et les cartes des deux côtés seront réservées. Si l’autre partie la modifie, elle repassera en attente et devra être acceptée à nouveau."
        confirm-label="Confirmer l’acceptation"
        tone="cyan"
        @cancel="modal = null"
        @confirm="
          run(() => acceptTrade(tradeId));
          modal = null;
        "
      />
      <TradeConfirmModal
        v-else-if="modal?.kind === 'modify'"
        title="Modifier la transaction ?"
        body="Au moins une partie a déjà accepté. La modifier va libérer les cartes réservées, annuler les acceptations et relancer la négociation."
        confirm-label="Modifier quand même"
        tone="down"
        @cancel="modal = null"
        @confirm="
          modal.run();
          modal = null;
        "
      />
      <TradeConfirmModal
        v-else-if="modal?.kind === 'abandon'"
        title="Abandonner l’échange ?"
        body="La transaction sera définitivement abandonnée et les cartes réservées libérées. Cette action est irréversible."
        confirm-label="Abandonner"
        tone="down"
        @cancel="modal = null"
        @confirm="
          run(() => abandonTrade(tradeId));
          modal = null;
        "
      />
    </template>
  </div>
</template>
