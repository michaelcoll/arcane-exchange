<script setup lang="ts">
import type { TradeStatus } from '~/utils/trade';

const props = defineProps<{ status: TradeStatus }>();

const abandoned = computed(() => props.status === 'ABANDONED');
const currentIndex = computed(() => TRADE_LIFECYCLE.findIndex((s) => s.status === props.status));

type NodeState = 'done' | 'cur' | 'todo';

const stateOf = (i: number): NodeState => {
  if (abandoned.value) return 'todo';
  if (i < currentIndex.value) return 'done';
  return i === currentIndex.value ? 'cur' : 'todo';
};

const dotClasses: Record<NodeState, string> = {
  done: 'border-cyan-500/40 bg-cyan-500/15 text-cyan-600 dark:border-cyan-400/40 dark:bg-cyan-400/15 dark:text-cyan-400',
  cur: 'border-transparent bg-cyan-500 text-zinc-950 shadow-[0_0_0_4px_var(--cyan-fill),0_0_22px_-6px_var(--cyan-glow)] dark:bg-cyan-400',
  todo: 'border-slate-300 bg-slate-100 text-slate-400 dark:border-white/15 dark:bg-white/5 dark:text-slate-500',
};

const labelClasses: Record<NodeState, string> = {
  done: 'text-slate-600 dark:text-slate-300',
  cur: 'text-cyan-600 dark:text-cyan-400',
  todo: 'text-slate-400 dark:text-slate-500',
};
</script>

<template>
  <div :class="['flex w-full items-start', abandoned ? 'opacity-45 grayscale-[0.6]' : '']">
    <div
      v-for="(step, i) in TRADE_LIFECYCLE"
      :key="step.status"
      class="relative flex min-w-0 flex-1 flex-col items-center gap-2.5"
    >
      <span
        v-if="i < TRADE_LIFECYCLE.length - 1"
        :class="[
          'absolute top-[15px] left-[calc(50%+15px)] z-[1] h-0.5 w-[calc(100%-30px)] max-[620px]:top-[13px] max-[620px]:left-[calc(50%+13px)] max-[620px]:w-[calc(100%-26px)]',
          stateOf(i) === 'done'
            ? 'bg-cyan-500/40 dark:bg-cyan-400/40'
            : 'bg-slate-200 dark:bg-white/10',
        ]"
      />
      <span
        :class="[
          'relative z-[2] grid h-[30px] w-[30px] place-items-center rounded-full border font-mono text-xs font-bold transition-all duration-300 ease-[cubic-bezier(0.4,0,0.1,1)] max-[620px]:h-[26px] max-[620px]:w-[26px] max-[620px]:text-[11px]',
          dotClasses[stateOf(i)],
        ]"
      >
        <Icon v-if="stateOf(i) === 'done'" name="lucide:check" size="13" />
        <template v-else>{{ i + 1 }}</template>
      </span>
      <span
        :class="[
          'text-center text-[11px] leading-tight font-semibold max-[620px]:text-[9px]',
          labelClasses[stateOf(i)],
        ]"
        >{{ step.label }}</span
      >
    </div>
  </div>
</template>
