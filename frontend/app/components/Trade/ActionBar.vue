<script setup lang="ts">
import type { TradePrimaryAction } from '~/composables/useTradeView';

/* Barre d'action mobile collée en bas, comme `TradeActionBar` côté iOS : une seule décision par
 * statut, jamais un mur de boutons. Abandonner vit dans le menu « … » de la barre de page.
 * Elle rend l'action principale que la vue du Trade a choisie, sans rien redécider ; aucune barre
 * quand il n'y a plus d'action. */
defineProps<{
  action: TradePrimaryAction;
  partner: string;
  busy: boolean;
}>();

const emit = defineEmits<{ accept: []; confirm: [] }>();

const button =
  'inline-flex w-full items-center justify-center gap-2 rounded-xl border border-transparent px-4 py-3 text-[15px] leading-none font-bold whitespace-nowrap shadow-lg transition-all duration-150 active:translate-y-px disabled:pointer-events-none disabled:opacity-50';
const waiting = 'flex items-center justify-center gap-2 py-3 text-sm font-medium';
</script>

<template>
  <div
    v-if="action.kind !== 'none'"
    data-part="bar"
    class="fixed inset-x-0 bottom-0 z-50 border-t border-[var(--line)] bg-slate-100/80 px-4 pt-2.5 pb-[calc(0.625rem+env(safe-area-inset-bottom))] backdrop-blur-md dark:bg-zinc-950/80"
  >
    <button
      v-if="action.kind === 'accept'"
      type="button"
      :class="[button, 'bg-primary hover:bg-primary-soft text-[var(--on-primary)]']"
      :disabled="busy"
      @click="emit('accept')"
    >
      <Icon name="lucide:check" :size="17" />{{ action.label }}
    </button>

    <div v-else-if="action.kind === 'awaitingAcceptance'" :class="[waiting, 'text-[var(--ink-3)]']">
      <Icon name="lucide:clock" :size="15" />
      <span
        >En attente de <span class="text-secondary">@{{ partner }}</span></span
      >
    </div>

    <button
      v-else-if="action.kind === 'confirmExchange'"
      type="button"
      :class="[button, 'bg-secondary hover:bg-secondary-soft text-[var(--on-secondary)]']"
      :disabled="busy"
      @click="emit('confirm')"
    >
      <Icon name="lucide:check" :size="17" />Confirmer « échange réalisé »
    </button>

    <div v-else :class="[waiting, 'text-secondary']">
      <Icon name="lucide:check" :size="15" />
      <span>Confirmé, en attente de @{{ partner }}</span>
    </div>
  </div>
</template>
