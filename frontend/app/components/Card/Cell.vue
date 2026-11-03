<script setup lang="ts">
import type { PurchaseSpreadKind } from '~/utils/purchase-spread';

const props = withDefaults(
  defineProps<{
    imageUrl?: string | null;
    name: string;
    qty?: number;
    price?: number;
    purchased?: number;
    trend?: number;
    clickable?: boolean;
    foil?: boolean;
    size?: 'sm' | 'md' | 'lg';
    ownerCount?: number;
    reserved?: boolean;
  }>(),
  {
    clickable: true,
  },
);

const emit = defineEmits(['click']);

const hasPurchase = computed(() => props.purchased !== undefined);

const spread = computed(() =>
  hasPurchase.value ? computePurchaseSpread(props.purchased, props.trend) : null,
);

const spreadTagClass = computed(() => {
  if (!spread.value) return '';
  const kinds: Record<PurchaseSpreadKind, string> = {
    gain: 'text-emerald-600 dark:text-emerald-400 bg-emerald-500/15',
    loss: 'text-red-500 dark:text-red-400 bg-red-500/15',
    par: 'text-slate-400 dark:text-slate-500',
  };
  return `font-mono text-2xs font-bold px-1.5 py-px rounded ${kinds[spread.value.kind]}`;
});
</script>

<template>
  <div class="flex flex-col gap-2">
    <MtgCard
      :qty="qty"
      :image-url="imageUrl"
      :name="name"
      :clickable="clickable ?? true"
      :foil="foil"
      :size="size"
      :reserved="reserved"
      @click="emit('click')"
    />
    <div class="flex flex-col gap-1">
      <span
        class="overflow-hidden text-xs leading-tight font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
        >{{ name }}</span
      >

      <div class="flex items-center justify-between gap-2">
        <span
          v-if="!hasPurchase"
          class="font-mono text-xs font-semibold text-slate-800 dark:text-slate-100"
        >
          <template v-if="price != null">{{ formatPrice(price) }}</template>
        </span>

        <!-- compare: trend price + unit crossed out + % badge -->
        <template v-else-if="spread">
          <span
            v-if="spread.kind === 'par'"
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
            <span :class="spreadTagClass">{{ spread.sign }}{{ spread.abs }}%</span>
          </span>
        </template>
        <span v-else class="font-mono text-xs font-semibold text-slate-800 dark:text-slate-100">{{
          formatPrice(purchased)
        }}</span>

        <span
          v-if="ownerCount != null && ownerCount > 1"
          class="text-2xs inline-flex items-center gap-1 font-mono whitespace-nowrap text-slate-400 dark:text-slate-500"
        >
          <Icon name="lucide:user" :size="11" class="shrink-0" />
          {{ ownerCount }}
        </span>
      </div>
    </div>
  </div>
</template>
