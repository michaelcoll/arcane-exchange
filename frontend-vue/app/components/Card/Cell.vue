<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    scryfallId?: string;
    theGathererId?: string;
    name: string;
    qty?: number;
    price?: number;
    purchased?: number;
    trend?: number;
    deal?: 'none' | 'compare';
    clickable?: boolean;
    foil?: boolean;
    size?: 'sm' | 'md' | 'lg';
  }>(),
  {
    clickable: true,
  },
);

const emit = defineEmits(['click']);

const dealInfo = computed(() => {
  if (!props.deal || props.deal === 'none') return null;
  const u = props.purchased;
  const t = props.trend;
  if (u == null || t == null) return null;
  const pct = Math.round(((t - u) / u) * 100);
  const kind = pct >= 3 ? 'good' : pct <= -3 ? 'bad' : 'par';
  return { pct, kind, abs: Math.abs(pct), sign: pct <= 0 ? '−' : '+' };
});

const dealTagClass = computed(() => {
  if (!dealInfo.value) return '';
  const kinds: Record<string, string> = {
    good: 'text-emerald-600 dark:text-emerald-400 bg-emerald-500/15',
    bad: 'text-red-500 dark:text-red-400 bg-red-500/15',
    par: 'text-slate-400 dark:text-slate-500',
  };
  return `font-mono text-2xs font-bold px-1.5 py-px rounded ${kinds[dealInfo.value.kind] ?? ''}`;
});
</script>

<template>
  <div class="flex flex-col gap-2">
    <MtgCard
      :qty="qty"
      :scryfall-id="scryfallId"
      :the-gatherer-id="theGathererId"
      :name="name"
      :clickable="clickable ?? true"
      :foil="foil"
      :size="size"
      @click="emit('click')"
    />
    <div class="flex flex-col gap-1">
      <span
        class="overflow-hidden text-xs leading-tight font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
        >{{ name }}</span
      >

      <!-- no deal -->
      <span
        v-if="!deal || deal === 'none'"
        class="font-mono text-xs font-semibold text-slate-800 dark:text-slate-100"
      >
        <template v-if="price != null">{{ formatPrice(price) }}</template>
      </span>

      <!-- compare: trend price + unit crossed out + % badge -->
      <template v-else-if="deal === 'compare' && dealInfo">
        <span
          v-if="dealInfo.kind === 'par'"
          class="font-mono text-xs font-semibold text-slate-800 dark:text-slate-100"
          >{{ formatPrice(trend) }}</span
        >
        <span v-else class="flex flex-wrap items-center gap-1.5">
          <span class="font-mono text-xs font-semibold text-slate-800 dark:text-slate-100">{{
            formatPrice(trend)
          }}</span>
          <span
            v-if="size !== 'sm'"
            class="font-mono text-xs text-slate-400 line-through dark:text-slate-500"
            >{{ formatPrice(purchased) }}</span
          >
          <span :class="dealTagClass">{{ dealInfo.sign }}{{ dealInfo.abs }}%</span>
        </span>
      </template>
      <span
        v-else-if="deal === 'compare'"
        class="font-mono text-xs font-semibold text-slate-800 dark:text-slate-100"
      >
        <template v-if="purchased != null">{{ formatPrice(purchased) }}</template>
        <template v-else-if="price != null">{{ formatPrice(price) }}</template>
      </span>
    </div>
  </div>
</template>
