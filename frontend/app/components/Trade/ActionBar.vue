<script setup lang="ts">
import type { TradeStatus } from '~/utils/trade';

/* Barre d'action mobile collée en bas, comme `TradeActionBar` côté iOS : une seule décision par
 * statut, jamais un mur de boutons. Abandonner vit dans le menu « … » de la barre de page.
 * Aucune barre une fois l'échange réalisé, clôturé ou abandonné. */
const props = defineProps<{
  status: TradeStatus;
  meAccepted: boolean;
  meConfirmed: boolean;
  partner: string;
  /** Total reçu − total donné, en centimes : porte le règlement sur le bouton d'acceptation. */
  diff: number;
  busy: boolean;
}>();

const emit = defineEmits<{ accept: []; confirm: [] }>();

const negotiating = computed(() => props.status === 'PENDING' || props.status === 'ONE_ACCEPTED');
const locked = computed(() => props.status === 'FULLY_ACCEPTED');

const button =
  'inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-3 text-[15px] leading-none font-bold whitespace-nowrap shadow-lg transition-all duration-150 active:translate-y-px disabled:pointer-events-none disabled:opacity-50';
const waiting = 'flex items-center justify-center gap-2 py-3 text-sm font-medium';
</script>

<template>
  <div
    v-if="negotiating || locked"
    data-part="bar"
    class="fixed inset-x-0 bottom-0 z-50 border-t border-[var(--line)] bg-slate-100/80 px-4 pt-2.5 pb-[calc(0.625rem+env(safe-area-inset-bottom))] backdrop-blur-md dark:bg-zinc-950/80"
  >
    <template v-if="negotiating">
      <div v-if="meAccepted" :class="[waiting, 'text-[var(--ink-3)]']">
        <Icon name="lucide:clock" :size="15" />
        <span
          >En attente de <span class="text-secondary">@{{ partner }}</span></span
        >
      </div>
      <button
        v-else
        type="button"
        :class="[button, 'bg-primary hover:bg-primary-soft text-[var(--on-primary)]']"
        :disabled="busy"
        @click="emit('accept')"
      >
        <Icon name="lucide:check" :size="17" />{{ tradeAcceptLabel(diff) }}
      </button>
    </template>

    <template v-else>
      <div v-if="meConfirmed" :class="[waiting, 'text-secondary']">
        <Icon name="lucide:check" :size="15" />
        <span>Confirmé, en attente de @{{ partner }}</span>
      </div>
      <button
        v-else
        type="button"
        :class="[button, 'bg-secondary hover:bg-secondary-soft text-[var(--on-secondary)]']"
        :disabled="busy"
        @click="emit('confirm')"
      >
        <Icon name="lucide:check" :size="17" />Confirmer « échange réalisé »
      </button>
    </template>
  </div>
</template>
