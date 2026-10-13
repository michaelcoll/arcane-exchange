<script setup lang="ts">
import type { TradeView } from '~/composables/useTradeView';

/* Desktop action panel of the Trade screen. It displays what the view of the Trade decided
 * (primary action, rating, abandon) with the words of mobile, and never reads the Trade status.
 * Its only decision of its own is the `hint` below, picked from the flags and the primary action
 * of the view. Only the layout differs from the mobile action bar, and the abandon is a button
 * here rather than an entry of the « … » menu. */
const props = defineProps<{
  /** The view of the Trade, of which the panel reads only what concerns it. */
  view: Pick<
    TradeView,
    | 'partnerUsername'
    | 'editable'
    | 'reserved'
    | 'abandonable'
    | 'primaryAction'
    | 'rating'
    | 'busy'
  >;
}>();

const emit = defineEmits<{
  accept: [];
  confirm: [];
  abandon: [];
  rate: [value: number];
  skip: [];
}>();

/* Reminder of the rule in force, on the left of the buttons. The view carries no text for it, so
 * the panel derives one of four from the view: `editable` with or without `reserved` while cards
 * can still change, then the kind of the primary action for a locked Trade, and the closing
 * sentence for everything else. */
const hint = computed(() => {
  const { editable, reserved, primaryAction } = props.view;
  if (editable) {
    return reserved
      ? 'Cartes réservées · modifiable (repasse en négociation)'
      : 'Modifiable par les deux parties · aucune carte réservée';
  }
  if (primaryAction.kind === 'confirmExchange' || primaryAction.kind === 'awaitingConfirmation') {
    return 'Échange physique en personne · confirmez chacun une fois réalisé';
  }
  return 'Cette transaction est close.';
});

const btnBase =
  'inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap transition-all duration-150 hover:-translate-y-px active:translate-y-0 disabled:pointer-events-none disabled:opacity-50';
const btnDanger = `${btnBase} border border-red-500/40 bg-transparent text-red-600 hover:bg-red-500/10 dark:border-red-400/40 dark:text-red-400 dark:hover:bg-red-400/10`;
const btnPrimary = `${btnBase} border border-transparent bg-primary font-bold text-[var(--on-primary)] shadow-lg hover:bg-primary-soft`;
const waiting =
  'inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap';
</script>

<template>
  <div
    class="rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
  >
    <!-- The mobile rating section, without its own background: the panel already frames it. -->
    <TradeRatingSection
      v-if="view.rating.visible"
      class="!rounded-none !bg-transparent !p-0"
      :partner="view.partnerUsername"
      :rating="view.rating"
      :busy="view.busy"
      @rate="emit('rate', $event)"
      @skip="emit('skip')"
    />

    <div v-else class="flex flex-wrap items-center justify-between gap-3.5">
      <span
        data-part="hint"
        class="inline-flex items-center gap-1.5 text-xs text-slate-400 dark:text-slate-500"
      >
        <Icon
          v-if="view.editable && view.reserved"
          name="lucide:lock"
          size="12"
          class="text-secondary"
        />
        {{ hint }}
      </span>

      <div class="flex flex-wrap items-center gap-2.5">
        <button
          v-if="view.abandonable"
          type="button"
          :class="btnDanger"
          :disabled="view.busy"
          @click="emit('abandon')"
        >
          Abandonner
        </button>

        <span v-if="view.primaryAction.kind !== 'none'" data-part="action" class="inline-flex">
          <button
            v-if="view.primaryAction.kind === 'accept'"
            type="button"
            :class="btnPrimary"
            :disabled="view.busy"
            @click="emit('accept')"
          >
            <Icon name="lucide:check" size="16" />{{ view.primaryAction.label }}
          </button>

          <span
            v-else-if="view.primaryAction.kind === 'awaitingAcceptance'"
            :class="[
              waiting,
              'border-slate-200 bg-slate-400/10 text-slate-500 dark:border-white/10 dark:text-slate-400',
            ]"
          >
            <Icon name="lucide:clock" size="13" />
            <span
              >En attente de <span class="text-secondary">@{{ view.partnerUsername }}</span></span
            >
          </span>

          <button
            v-else-if="view.primaryAction.kind === 'confirmExchange'"
            type="button"
            :class="btnPrimary"
            :disabled="view.busy"
            @click="emit('confirm')"
          >
            <Icon name="lucide:check" size="16" />Confirmer « échange réalisé »
          </button>

          <span v-else :class="[waiting, 'border-secondary/30 bg-secondary/10 text-secondary-ink']">
            <Icon name="lucide:check" size="13" />
            <span>Confirmé, en attente de @{{ view.partnerUsername }}</span>
          </span>
        </span>
      </div>
    </div>
  </div>
</template>
