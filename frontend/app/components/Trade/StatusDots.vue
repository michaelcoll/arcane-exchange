<script setup lang="ts">
import type { TradeStatusParam } from '~/bindings/TradeStatusParam';
import type { TradeViewSteps } from '~/composables/useTradeView';

/* Où en est l'échange, comme `TradeStatusIndicator` côté iOS : les étapes de la vue du Trade en
 * points, celle en cours gonflée en pastille à sa propre place. Sans étape en cours (Trade
 * abandonné, hors parcours) : points éteints, pastille en fin de ligne, sans « n/5 ». */
const props = defineProps<{ status: TradeStatusParam; steps: TradeViewSteps }>();

const emit = defineEmits<{ help: [] }>();

const count = computed(() => props.steps.list.length);

type Part = { kind: 'done' | 'todo'; key: string } | { kind: 'pill'; key: string };

/* Étapes franchies, pastille, étapes à venir — ou, hors parcours, les points éteints puis la
 * pastille. */
const parts = computed<Part[]>(() => {
  const pill: Part = { kind: 'pill', key: 'pill' };
  const dots = props.steps.list.map((step): Part => {
    if (step.state === 'current') return pill;
    return { kind: step.state === 'done' ? 'done' : 'todo', key: step.status };
  });
  return props.steps.currentIndex === null ? [...dots, pill] : dots;
});

/* « NÉGOCIATION · 1/5 », ou « ABANDONNÉ » hors parcours. */
const pillLabel = computed(() => {
  const index = props.steps.currentIndex;
  if (index === null) return 'ABANDONNÉ';
  return `${props.steps.list[index]!.title} · ${index + 1}/${count.value}`.toUpperCase();
});

const pillClasses: Record<TradeStatusParam, string> = {
  PENDING: 'border-primary/40 bg-primary/15 text-primary-ink',
  ONE_ACCEPTED: 'border-primary/40 bg-primary/15 text-primary-ink',
  FULLY_ACCEPTED: 'border-secondary/40 bg-secondary/15 text-secondary-ink',
  COMPLETED:
    'border-emerald-500/40 bg-emerald-500/15 text-emerald-700 dark:border-emerald-400/40 dark:bg-emerald-400/15 dark:text-emerald-300',
  CLOSED:
    'border-emerald-500/40 bg-emerald-500/15 text-emerald-700 dark:border-emerald-400/40 dark:bg-emerald-400/15 dark:text-emerald-300',
  ABANDONED:
    'border-red-500/40 bg-red-500/15 text-red-600 dark:border-red-400/40 dark:bg-red-400/15 dark:text-red-400',
};

const accessibleLabel = computed(() => {
  const label = TRADE_STATUS_META[props.status].label;
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
          :class="[
            'rounded-full border px-[11px] py-1 font-mono text-[10px] font-medium tracking-[0.1em] whitespace-nowrap',
            pillClasses[status],
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
