<script setup lang="ts">
import type { TradeCard, TradeRating, TradeStatus, TradeValueMode } from '~/utils/trade';

definePageMeta({ middleware: 'auth' });

const COUNTERPARTY = { name: '@mizzix_42', initials: 'M4', rating: 4.8 };

const mode = ref<TradeValueMode>('eur');

const give = ref<TradeCard[]>([
  { name: 'Black Market Connections', eur: 13, edh: 18 },
  { name: 'Reprieve', eur: 4, edh: 7 },
  { name: 'Persistent Constrictor', eur: 2, edh: 12 },
]);
const get = ref<TradeCard[]>([
  { name: 'Sire of Seven Deaths', eur: 31, edh: 21 },
  { name: 'The Soul Stone', eur: 9, edh: 5 },
]);

/* ---------- machine à états (cf. .agents/trade-workflow.instructions.md) ---------- */
const acceptedMe = ref(false);
const acceptedThem = ref(false);
const confirmedMe = ref(false);
const confirmedThem = ref(false);
const ratingMe = ref<TradeRating>(null);
const ratingThem = ref<TradeRating>(null);
const abandonedBy = ref<'me' | 'them' | null>(null);

const status = computed<TradeStatus>(() => {
  if (abandonedBy.value) return 'ABANDONED';
  if (confirmedMe.value && confirmedThem.value)
    return ratingMe.value != null && ratingThem.value != null ? 'CLOSED' : 'COMPLETED';
  if (acceptedMe.value && acceptedThem.value) return 'FULLY_ACCEPTED';
  if (acceptedMe.value || acceptedThem.value) return 'ONE_ACCEPTED';
  return 'PENDING';
});

const editable = computed(() => isTradeEditable(status.value));
const reserved = computed(() => isTradeReserved(status.value));

/* ---------- valeurs ---------- */
const sum = (cards: TradeCard[]) =>
  cards.reduce((s, c) => s + (mode.value === 'eur' ? c.eur : c.edh), 0);

const giveTotal = computed(() => sum(give.value));
const getTotal = computed(() => sum(get.value));
const diff = computed(() => getTotal.value - giveTotal.value);

const giveEurSum = computed(() => give.value.reduce((s, c) => s + c.eur, 0));
const getEurSum = computed(() => get.value.reduce((s, c) => s + c.eur, 0));

const modeOptions = [
  { value: 'eur', label: 'Prix €', tone: 'cyan' },
  { value: 'edh', label: 'EDHREC %', tone: 'vio' },
];

/* ---------- modales ---------- */
type Modal = { kind: 'accept' } | { kind: 'abandon' } | { kind: 'modify'; run?: () => void };
const modal = ref<Modal | null>(null);

/* Toute modification après une première acceptation libère les cartes,
 * annule les acceptations et relance la négociation → confirmation requise. */
const tryModify = (run: () => void) => {
  if (status.value === 'ONE_ACCEPTED') {
    modal.value = { kind: 'modify', run };
    return;
  }
  run();
};

const applyModification = (run?: () => void) => {
  run?.();
  acceptedMe.value = false;
  acceptedThem.value = false;
};

const removeGive = (i: number) => tryModify(() => give.value.splice(i, 1));
const removeGet = (i: number) => tryModify(() => get.value.splice(i, 1));
const addGive = () => tryModify(() => navigateTo('/collection'));
const addGet = () => tryModify(() => navigateTo('/search'));

const confirmExchange = () => {
  confirmedMe.value = true;
};
const rate = (value: TradeRating) => {
  ratingMe.value = value;
};

const formatRating = (r: TradeRating) => (r == null || r === 'skip' ? 'passée' : `${r}/5`);

/* ---------- styles de boutons partagés ---------- */
const btnBase =
  'inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap transition-all duration-150 hover:-translate-y-px active:translate-y-0';
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
          <PlayerAvatar :initials="COUNTERPARTY.initials" />
          <div class="flex flex-col gap-px">
            <h2 class="font-display text-base font-semibold tracking-tight">
              Échange avec
              <span class="text-cyan-600 dark:text-cyan-400">{{ COUNTERPARTY.name }}</span>
            </h2>
            <span :class="hint">
              <Icon
                name="mdi:star"
                size="11"
                class="align-[-1px] text-violet-500 dark:text-violet-300"
              />
              {{ COUNTERPARTY.rating.toLocaleString('fr-FR') }}
            </span>
          </div>
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
      :counterparty="COUNTERPARTY.name"
      :accepted="acceptedMe"
      :confirmed="confirmedMe"
      :abandoned-by-me="abandonedBy === 'me'"
    />

    <!-- EDHREC INFO BANNER -->
    <div
      v-if="mode === 'edh'"
      class="mb-4 flex items-start gap-2.5 rounded-2xl border border-violet-500/30 bg-violet-500/10 px-4 py-3 shadow-lg backdrop-blur-md dark:border-violet-400/30 dark:bg-violet-400/10"
    >
      <span class="mt-px text-violet-500 dark:text-violet-300"
        ><Icon name="lucide:info" size="16"
      /></span>
      <span class="text-sm text-slate-600 dark:text-slate-300">
        Le <span class="text-violet-500 dark:text-violet-300">% EDHREC</span> = part des decks
        référencés qui jouent la carte. On compare la désirabilité plutôt que l'euro.
      </span>
    </div>

    <!-- SÉLECTEUR DE MODE -->
    <div class="mb-3.5 flex flex-wrap items-center justify-between gap-2.5">
      <span :class="label">Comparaison de valeur</span>
      <SegToggle v-model="mode" :options="modeOptions" />
    </div>

    <!-- COLONNES + BALANCE -->
    <div
      class="grid [grid-template-columns:1fr_auto_1fr] items-stretch gap-4 max-md:[grid-template-columns:1fr]"
    >
      <TradeColumn
        label="Je donne"
        :cards="give"
        :mode="mode"
        accent="neutral"
        :editable="editable"
        :reserved="reserved"
        add-label="Ajouter une de mes cartes"
        @remove="removeGive"
        @add="addGive"
      />

      <div class="flex min-w-[168px] flex-col items-center justify-center gap-3.5">
        <TradeBalance :diff="diff" :give-total="giveTotal" :get-total="getTotal" :mode="mode" />
        <span
          v-if="mode === 'edh'"
          class="inline-flex items-center gap-1.5 rounded-full border border-violet-500/30 bg-violet-500/10 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-violet-700 select-none dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300"
        >
          <Icon name="lucide:refresh-cw" size="12" />
          €{{ giveEurSum }} ↔ €{{ getEurSum }}
        </span>
        <span
          class="text-2xs inline-flex items-center gap-1.5 font-mono tracking-wide text-slate-400 dark:text-slate-500"
        >
          <Icon name="lucide:info" size="13" /> Réglé hors plateforme
        </span>
      </div>

      <TradeColumn
        label="Je reçois"
        :cards="get"
        :mode="mode"
        accent="cyan"
        :editable="editable"
        :reserved="reserved"
        add-label="Chercher dans sa collection"
        @remove="removeGet"
        @add="addGet"
      />
    </div>

    <!-- ACTIONS SELON LE STATUT -->
    <div :class="[panel, 'mt-[18px] p-4']">
      <!-- PENDING -->
      <div v-if="status === 'PENDING'" class="flex flex-wrap items-center justify-between gap-3.5">
        <span :class="hint">Modifiable par les deux parties · aucune carte réservée</span>
        <div class="flex flex-wrap items-center gap-2.5">
          <button :class="btnDanger" @click="modal = { kind: 'abandon' }">Abandonner</button>
          <button :class="btnPrimary" @click="modal = { kind: 'accept' }">
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
          <button :class="btnDanger" @click="modal = { kind: 'abandon' }">Abandonner</button>
          <button :class="btnGhost" @click="modal = { kind: 'modify' }">Modifier</button>
          <button v-if="!acceptedMe" :class="btnPrimary" @click="modal = { kind: 'accept' }">
            <Icon name="lucide:check" size="16" /> Accepter à mon tour
          </button>
        </div>
      </div>

      <!-- FULLY_ACCEPTED -->
      <div
        v-else-if="status === 'FULLY_ACCEPTED'"
        class="flex flex-wrap items-center justify-between gap-3.5"
      >
        <span :class="hint">Échange physique en personne · confirmez chacun une fois réalisé</span>
        <div class="flex flex-wrap items-center gap-2.5">
          <button :class="btnDanger" @click="modal = { kind: 'abandon' }">Abandonner</button>
          <span
            v-if="confirmedMe"
            class="inline-flex items-center gap-1.5 rounded-full border border-violet-500/30 bg-violet-500/10 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-violet-700 dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300"
          >
            <Icon name="lucide:check" size="13" /> Tu as confirmé · en attente de
            {{ COUNTERPARTY.name }}
          </span>
          <button v-else :class="btnPrimary" @click="confirmExchange">
            <Icon name="lucide:check" size="16" /> Confirmer « échange réalisé »
          </button>
        </div>
      </div>

      <!-- COMPLETED -->
      <div v-else-if="status === 'COMPLETED'" class="flex flex-col gap-3.5">
        <div class="flex flex-wrap items-center justify-between gap-3.5">
          <div class="flex flex-col gap-0.5">
            <span :class="label">Noter {{ COUNTERPARTY.name }}</span>
            <span :class="hint">Optionnel · 0 à 5 étoiles</span>
          </div>
          <TradeRatingStars v-if="ratingMe == null" :value="null" @rate="rate" />
          <span
            v-else
            class="inline-flex items-center gap-1.5 rounded-full border border-violet-500/30 bg-violet-500/10 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-violet-700 dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300"
          >
            <template v-if="ratingMe === 'skip'">Notation passée</template>
            <template v-else>
              <Icon name="mdi:star" size="13" /> Tu as mis {{ ratingMe }}/5
            </template>
          </span>
        </div>
        <div class="h-px bg-slate-200 dark:bg-white/10" />
        <div class="flex flex-wrap items-center justify-between gap-3">
          <span :class="hint">
            {{
              ratingThem != null
                ? `${COUNTERPARTY.name} a noté de son côté.`
                : `En attente éventuelle de la note de ${COUNTERPARTY.name}.`
            }}
          </span>
          <button v-if="ratingMe == null" :class="btnGhost" @click="rate('skip')">
            Passer la notation
          </button>
        </div>
      </div>

      <!-- CLOSED -->
      <div v-else-if="status === 'CLOSED'" class="flex flex-wrap items-center gap-x-[18px] gap-y-2">
        <span :class="hint"
          >Ta note :
          <span class="font-semibold text-violet-500 dark:text-violet-300">{{
            formatRating(ratingMe)
          }}</span></span
        >
        <span :class="hint"
          >Note de {{ COUNTERPARTY.name }} :
          <span class="font-semibold text-violet-500 dark:text-violet-300">{{
            formatRating(ratingThem)
          }}</span></span
        >
      </div>

      <!-- ABANDONED -->
      <div v-else class="flex flex-wrap items-center justify-between gap-3.5">
        <div v-if="abandonedBy === 'them' && ratingMe == null" class="flex items-center gap-3">
          <span :class="hint">Noter {{ COUNTERPARTY.name }} (optionnel) :</span>
          <TradeRatingStars :value="null" @rate="rate" />
        </div>
        <span v-else :class="hint">Cette transaction est close.</span>
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
        acceptedMe = true;
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
        applyModification(modal?.kind === 'modify' ? modal.run : undefined);
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
        abandonedBy = 'me';
        modal = null;
      "
    />
  </div>
</template>
