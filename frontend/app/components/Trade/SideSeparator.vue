<script setup lang="ts">
/* Between the two sides of the Trade: "échange", or "échange réservé" once the cards are
 * reserved. Horizontal between two stacked rails (mobile), vertical between two columns (desktop). */
const props = withDefaults(
  defineProps<{
    reserved: boolean;
    orientation?: 'horizontal' | 'vertical';
  }>(),
  { orientation: 'horizontal' },
);

const line = computed(() => [
  'flex-1 bg-[var(--line-2)]',
  props.orientation === 'vertical' ? 'w-px' : 'h-px',
]);
</script>

<template>
  <div
    :class="[
      'flex items-center gap-2.5',
      orientation === 'vertical' ? 'flex-col py-0.5' : 'px-0.5',
    ]"
  >
    <span :class="line" />
    <span
      :class="[
        'inline-flex items-center gap-1.5 rounded-full border px-[13px] py-1.5 font-mono text-[11.5px] font-semibold whitespace-nowrap',
        reserved
          ? 'border-secondary/40 bg-secondary/15 text-secondary-ink'
          : 'border-[var(--line-2)] bg-slate-400/10 text-[var(--ink-2)]',
      ]"
    >
      <Icon :name="reserved ? 'lucide:lock' : 'lucide:arrow-left-right'" :size="12" />
      {{ reserved ? 'échange réservé' : 'échange' }}
    </span>
    <span :class="line" />
  </div>
</template>
