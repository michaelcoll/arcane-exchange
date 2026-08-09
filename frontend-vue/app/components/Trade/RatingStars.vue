<script setup lang="ts">
import type { TradeRating } from '~/utils/trade';

const props = withDefaults(defineProps<{ value: TradeRating; readOnly?: boolean }>(), {
  readOnly: false,
});

const emit = defineEmits<{ rate: [value: number] }>();

const hover = ref(0);
const filledUpTo = computed(
  () => hover.value || (typeof props.value === 'number' ? props.value : 0),
);
</script>

<template>
  <div class="inline-flex gap-[5px]" @mouseleave="hover = 0">
    <button
      v-for="i in 5"
      :key="i"
      type="button"
      :disabled="readOnly"
      :aria-label="`Noter ${i} sur 5`"
      :class="[
        'inline-flex transition-[transform,color] duration-150',
        readOnly ? 'cursor-default' : 'cursor-pointer hover:scale-[1.15]',
        i <= filledUpTo
          ? 'text-violet-500 drop-shadow-[0_0_6px_var(--violet-glow)] dark:text-violet-300'
          : 'text-slate-400 dark:text-slate-500',
      ]"
      @mouseenter="!readOnly && (hover = i)"
      @click="!readOnly && emit('rate', i)"
    >
      <Icon :name="i <= filledUpTo ? 'mdi:star' : 'mdi:star-outline'" size="22" />
    </button>
  </div>
</template>
