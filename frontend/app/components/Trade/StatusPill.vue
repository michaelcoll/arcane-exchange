<script setup lang="ts">
import type { TradeStatus } from '~/utils/trade';

const props = withDefaults(defineProps<{ status: TradeStatus; size?: 'sm' | 'md' }>(), {
  size: 'md',
});

const meta = computed(() => TRADE_STATUS_META[props.status]);
</script>

<template>
  <span
    :class="[
      'inline-flex items-center rounded-full border font-mono font-semibold tracking-wider whitespace-nowrap',
      size === 'sm' ? 'gap-1.5 px-2.5 py-[3px] text-[10px]' : 'text-2xs gap-[7px] px-3 py-[5px]',
      TRADE_TONE_CLASSES[meta.tone],
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
