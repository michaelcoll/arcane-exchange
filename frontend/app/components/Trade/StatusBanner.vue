<script setup lang="ts">
import type { TradeViewBanner } from '~/composables/useTradeView';
import type { TradeTone } from '~/utils/trade';

/* Le message de statut de la vue du Trade, affiché tel quel : ce composant ne décide ni du texte
 * ni du ton. */
defineProps<{ banner: TradeViewBanner }>();

const bannerClasses: Record<TradeTone, string> = {
  primary: 'border-slate-200 bg-slate-100 dark:border-white/15 dark:bg-white/5',
  secondary: 'border-secondary/30 bg-secondary/10',
  good: 'border-emerald-500/30 bg-emerald-500/10 dark:border-emerald-400/30 dark:bg-emerald-400/10',
  down: 'border-red-500/30 bg-red-500/10 dark:border-red-400/30 dark:bg-red-400/10',
  muted: 'border-slate-200 bg-slate-100 dark:border-white/15 dark:bg-white/5',
};

const iconClasses: Record<TradeTone, string> = {
  primary: 'border-primary/30 bg-primary/10 text-primary',
  secondary: 'border-secondary/40 bg-secondary/20 text-secondary',
  good: 'border-emerald-500/40 bg-transparent text-emerald-600 dark:border-emerald-400/40 dark:text-emerald-400',
  down: 'border-red-500/40 bg-transparent text-red-600 dark:border-red-400/40 dark:text-red-400',
  muted: 'border-slate-200 bg-slate-100 text-slate-500 dark:border-white/15 dark:bg-white/5',
};
</script>

<template>
  <div
    :data-tone="banner.tone"
    :class="['flex items-start gap-3 rounded-xl border p-4', bannerClasses[banner.tone]]"
  >
    <span
      :class="[
        'grid h-[34px] w-[34px] flex-none place-items-center rounded-lg border',
        iconClasses[banner.tone],
      ]"
    >
      <Icon :name="banner.icon" size="17" />
    </span>
    <div class="flex min-w-0 flex-col gap-0.5">
      <span class="text-sm font-semibold text-slate-800 dark:text-slate-100">{{
        banner.title
      }}</span>
      <span class="text-xs leading-relaxed text-slate-600 dark:text-slate-300">{{
        banner.text
      }}</span>
    </div>
  </div>
</template>
