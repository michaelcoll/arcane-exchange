<script setup lang="ts">
const props = defineProps<{
  /** `getTotal - giveTotal`, en centimes : > 0 tu reçois plus (tu dois la différence), < 0 l'inverse. */
  diff: number;
  giveTotal: number;
  getTotal: number;
}>();

const even = computed(() => Math.abs(props.diff) < 300);

const verdict = computed(() => {
  const abs = formatPrice(Math.abs(props.diff));
  if (even.value) return 'Équilibré';
  if (props.diff > 0) return `Tu dois ${abs}`;
  return `On te doit ${abs}`;
});

const getShare = computed(() => {
  const total = props.giveTotal + props.getTotal || 1;
  return (props.getTotal / total) * 100;
});
</script>

<template>
  <div class="flex w-full max-w-[200px] flex-col items-center gap-2">
    <div class="flex w-full items-baseline justify-between">
      <span
        class="text-2xs tracking-wide whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Donne</span
      >
      <span
        class="text-2xs tracking-wide whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Reçois</span
      >
    </div>
    <div
      class="relative flex h-3 w-full overflow-hidden rounded-full border border-slate-200 bg-slate-100 dark:border-white/10 dark:bg-zinc-800"
    >
      <span
        class="bg-secondary h-full transition-[width] duration-500 ease-out"
        :style="{ width: 100 - getShare + '%' }"
      />
      <span
        class="bg-primary h-full transition-[width] duration-500 ease-out"
        :style="{ width: getShare + '%' }"
      />
      <span
        class="absolute -top-0.5 -bottom-0.5 left-1/2 w-0.5 -translate-x-1/2 bg-slate-100 shadow-[0_0_0_1px_rgba(120,120,120,0.3)] dark:bg-zinc-950"
      />
    </div>
    <div class="flex w-full items-baseline justify-between">
      <span class="text-secondary font-mono text-sm font-semibold">{{
        formatPrice(giveTotal)
      }}</span>
      <span class="text-primary font-mono text-sm font-semibold">{{ formatPrice(getTotal) }}</span>
    </div>
    <div
      :class="[
        'rounded-xl border px-3 py-2 text-center font-mono text-sm font-semibold',
        even
          ? 'border-primary/30 bg-primary/10 text-primary-ink'
          : 'border-secondary/30 bg-secondary/10 text-secondary-ink',
      ]"
    >
      {{ verdict }}
    </div>
  </div>
</template>
