<script setup lang="ts">
import type { TradeStepState, TradeViewSteps } from '~/composables/useTradeView';

/* Le fil d'étapes desktop : il affiche les étapes de la vue du Trade, sans situer lui-même le
 * statut. La page ne le monte pas pour un Trade hors parcours (abandonné). */
defineProps<{ steps: TradeViewSteps }>();

const dotClasses: Record<TradeStepState, string> = {
  done: 'border-primary/40 bg-primary/15 text-primary',
  current:
    'border-transparent bg-primary text-[var(--on-primary)] shadow-[0_0_0_4px_var(--primary-fill),0_0_22px_-6px_var(--primary-glow)]',
  upcoming:
    'border-slate-300 bg-slate-100 text-slate-400 dark:border-white/15 dark:bg-white/5 dark:text-slate-500',
};

const labelClasses: Record<TradeStepState, string> = {
  done: 'text-slate-600 dark:text-slate-300',
  current: 'text-primary',
  upcoming: 'text-slate-400 dark:text-slate-500',
};
</script>

<template>
  <div class="flex w-full items-start">
    <div
      v-for="(step, i) in steps.list"
      :key="step.status"
      class="relative flex min-w-0 flex-1 flex-col items-center gap-2.5"
    >
      <span
        v-if="i < steps.list.length - 1"
        :class="[
          'absolute top-[15px] left-[calc(50%+15px)] z-[1] h-0.5 w-[calc(100%-30px)] max-[620px]:top-[13px] max-[620px]:left-[calc(50%+13px)] max-[620px]:w-[calc(100%-26px)]',
          step.state === 'done' ? 'bg-primary/40' : 'bg-slate-200 dark:bg-white/10',
        ]"
      />
      <span
        :class="[
          'relative z-[2] grid h-[30px] w-[30px] place-items-center rounded-full border font-mono text-xs font-bold transition-all duration-300 ease-[cubic-bezier(0.4,0,0.1,1)] max-[620px]:h-[26px] max-[620px]:w-[26px] max-[620px]:text-[11px]',
          dotClasses[step.state],
        ]"
      >
        <Icon v-if="step.state === 'done'" name="lucide:check" size="13" />
        <template v-else>{{ i + 1 }}</template>
      </span>
      <span
        :class="[
          'text-center text-[11px] leading-tight font-semibold max-[620px]:text-[9px]',
          labelClasses[step.state],
        ]"
        >{{ step.title }}</span
      >
    </div>
  </div>
</template>
