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
        class="h-full bg-violet-500 transition-[width] duration-500 ease-out dark:bg-violet-400"
        :style="{ width: 100 - getShare + '%' }"
      />
      <span
        class="h-full bg-cyan-500 transition-[width] duration-500 ease-out dark:bg-cyan-400"
        :style="{ width: getShare + '%' }"
      />
      <span
        class="absolute -top-0.5 -bottom-0.5 left-1/2 w-0.5 -translate-x-1/2 bg-slate-100 shadow-[0_0_0_1px_rgba(120,120,120,0.3)] dark:bg-zinc-950"
      />
    </div>
    <div class="flex w-full items-baseline justify-between">
      <span class="font-mono text-sm font-semibold text-violet-500 dark:text-violet-300">{{
        formatPrice(giveTotal)
      }}</span>
      <span class="font-mono text-sm font-semibold text-cyan-600 dark:text-cyan-400">{{
        formatPrice(getTotal)
      }}</span>
    </div>
    <div
      :class="[
        'rounded-xl border px-3 py-2 text-center font-mono text-sm font-semibold',
        even
          ? 'border-cyan-500/30 bg-cyan-500/10 text-cyan-700 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300'
          : 'border-violet-500/30 bg-violet-500/10 text-violet-700 dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300',
      ]"
    >
      {{ verdict }}
    </div>
  </div>
</template>
