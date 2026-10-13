<script setup lang="ts">
import type { TradeStepState, TradeViewSteps } from '~/composables/useTradeView';
import type { TradeStatusPresentation, TradeTone } from '~/utils/trade';

/* Où en est l'échange, comme `TradeStatusIndicator` côté iOS : les étapes de la vue du Trade en
 * points, celle en cours gonflée en pastille à sa propre place. Sans étape en cours (Trade
 * abandonné, hors parcours) : points éteints, pastille en fin de ligne, sans « n/5 ». */
const props = defineProps<{ presentation: TradeStatusPresentation; steps: TradeViewSteps }>();

const emit = defineEmits<{ help: [] }>();

const count = computed(() => props.steps.list.length);

type Part =
  { kind: Exclude<TradeStepState, 'current'>; key: string } | { kind: 'pill'; key: string };

/* Étapes franchies, pastille, étapes à venir — ou, hors parcours, les points éteints puis la
 * pastille. */
const parts = computed<Part[]>(() => {
  const pill: Part = { kind: 'pill', key: 'pill' };
  const dots = props.steps.list.map((step): Part => {
    if (step.state === 'current') return pill;
    return { kind: step.state, key: step.status };
  });
  return props.steps.currentIndex === null ? [...dots, pill] : dots;
});

/* « NÉGOCIATION · 1/5 », ou « ABANDONNÉ » hors parcours. */
const pillLabel = computed(() => {
  const index = props.steps.currentIndex;
  if (index === null) return 'ABANDONNÉ';
  return `${props.steps.list[index]!.title} · ${index + 1}/${count.value}`.toUpperCase();
});

/* La pastille prend le ton du statut, un cran plus soutenu que `Trade/StatusPill`. */
const pillClasses: Record<TradeTone, string> = {
  primary: 'border-primary/40 bg-primary/15 text-primary-ink',
  secondary: 'border-secondary/40 bg-secondary/15 text-secondary-ink',
  good: 'border-emerald-500/40 bg-emerald-500/15 text-emerald-700 dark:border-emerald-400/40 dark:bg-emerald-400/15 dark:text-emerald-300',
  down: 'border-red-500/40 bg-red-500/15 text-red-600 dark:border-red-400/40 dark:bg-red-400/15 dark:text-red-400',
  muted:
    'border-slate-300 bg-slate-100 text-slate-500 dark:border-white/15 dark:bg-white/5 dark:text-slate-400',
};

const accessibleLabel = computed(() => {
  const label = props.presentation.label;
  const index = props.steps.currentIndex;
  return index === null ? label : `${label}, étape ${index + 1} sur ${count.value}`;
});
</script>

<template>
  <div class="flex items-center justify-center gap-[7px]">
    <div role="img" :aria-label="accessibleLabel" class="flex items-center gap-[7px]">
      <template v-for="part in parts" :key="part.key">
        <span
          v-if="part.kind === 'pill'"
          data-part="pill"
          :data-tone="presentation.tone"
          :class="[
            'rounded-full border px-[11px] py-1 font-mono text-[10px] font-medium tracking-[0.1em] whitespace-nowrap',
            pillClasses[presentation.tone],
          ]"
          >{{ pillLabel }}</span
        >
        <!-- Le chemin parcouru reste primary quel que soit le ton du statut courant. -->
        <span
          v-else
          :data-dot="part.kind"
          :class="[
            'h-1.5 w-1.5 rounded-full',
            part.kind === 'done' ? 'bg-primary/40' : 'bg-slate-400/30',
          ]"
        />
      </template>
    </div>
    <button
      type="button"
      aria-label="Les étapes d'un échange"
      class="grid h-6 w-6 place-items-center rounded-full border border-[var(--line-2)] text-[var(--ink-3)] transition-colors duration-150 hover:text-[var(--ink)]"
      @click="emit('help')"
    >
      <Icon name="lucide:info" :size="13" />
    </button>
  </div>
</template>
