<script setup lang="ts">
import type { TradeStatus, TradeTone } from '~/utils/trade';

const props = withDefaults(defineProps<{ status: TradeStatus; size?: 'sm' | 'md' }>(), {
  size: 'md',
});

const meta = computed(() => TRADE_STATUS_META[props.status]);

const toneClasses: Record<TradeTone, string> = {
  cyan: 'border-cyan-500/30 bg-cyan-500/10 text-cyan-700 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300',
  violet:
    'border-violet-500/30 bg-violet-500/10 text-violet-700 dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300',
  good: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/30 dark:bg-emerald-400/10 dark:text-emerald-300',
  down: 'border-red-500/30 bg-red-500/10 text-red-600 dark:border-red-400/30 dark:bg-red-400/10 dark:text-red-400',
  muted:
    'border-slate-300 bg-slate-100 text-slate-500 dark:border-white/15 dark:bg-white/5 dark:text-slate-400',
};
</script>

<template>
  <span
    :class="[
      'inline-flex items-center rounded-full border font-mono font-semibold tracking-wider whitespace-nowrap',
      size === 'sm' ? 'gap-1.5 px-2.5 py-[3px] text-[10px]' : 'text-2xs gap-[7px] px-3 py-[5px]',
      toneClasses[meta.tone],
    ]"
  >
    <span
      :class="[
        'flex-none rounded-full bg-current',
        size === 'sm' ? 'h-1.5 w-1.5' : 'h-[7px] w-[7px]',
        meta.tone === 'muted' ? '' : 'shadow-[0_0_8px_currentColor]',
      ]"
    />
    {{ meta.label }}
  </span>
</template>
