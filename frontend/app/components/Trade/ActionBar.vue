<script setup lang="ts">
import type { TradePrimaryAction } from '~/composables/useTradeView';

/* The Trade action bar, like the iOS `TradeActionBar`: one decision per status, never a wall of
 * buttons. It renders the primary action the Trade view chose, without deciding anything again;
 * no bar once there is no action left.
 * On mobile, it sticks to the bottom and abandoning lives in the page bar's "…" menu. On desktop,
 * it sits inline under the sides and carries an × abandon button. */
const props = withDefaults(
  defineProps<{
    action: TradePrimaryAction;
    partner: string;
    busy: boolean;
    /** `sticky`: stuck to the bottom of the screen (mobile); `inline`: inline under the sides,
     * with the × abandon button (desktop). */
    layout?: 'sticky' | 'inline';
    /** The Trade can still be abandoned: only the `inline` layout uses it. */
    abandonable?: boolean;
  }>(),
  { layout: 'sticky', abandonable: false },
);

const emit = defineEmits<{ accept: []; confirm: []; abandon: [] }>();

const inline = computed(() => props.layout === 'inline');

const button = computed(() => [
  'inline-flex flex-1 items-center justify-center gap-2 border border-transparent px-4 leading-none font-bold whitespace-nowrap shadow-lg transition-all duration-150 active:translate-y-px disabled:pointer-events-none disabled:opacity-50',
  inline.value ? 'min-h-[46px] rounded-full text-[14.5px]' : 'w-full rounded-xl py-3 text-[15px]',
]);
const waiting = 'flex flex-1 items-center justify-center gap-2 py-3 text-sm font-medium';
</script>

<template>
  <div
    v-if="action.kind !== 'none'"
    data-part="bar"
    :class="
      inline
        ? 'mx-auto flex w-fit max-w-full min-w-[min(500px,100%)] gap-2 rounded-[30px] border border-[var(--line)] bg-white/60 p-2 shadow-lg backdrop-blur-md dark:bg-zinc-900/60'
        : 'fixed inset-x-0 bottom-0 z-50 flex border-t border-[var(--line)] bg-slate-100/80 px-4 pt-2.5 pb-[calc(0.625rem+env(safe-area-inset-bottom))] backdrop-blur-md dark:bg-zinc-950/80'
    "
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

    <button
      v-if="inline && abandonable"
      type="button"
      aria-label="Abandonner l'échange"
      title="Abandonner l'échange"
      class="grid h-[46px] w-[46px] flex-none place-items-center rounded-full border border-[var(--line)] bg-transparent text-[var(--ink-2)] transition-colors duration-150 hover:border-[var(--line-2)] hover:bg-red-500/10 hover:text-red-600 disabled:pointer-events-none disabled:opacity-50 dark:hover:text-red-400"
      :disabled="busy"
      @click="emit('abandon')"
    >
      <Icon name="lucide:x" :size="16" />
    </button>
  </div>
</template>
