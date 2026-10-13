<script setup lang="ts">
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';

const props = withDefaults(
  defineProps<{
    entries: PriceHistoryEntry[];
    pending?: boolean;
    detail?: boolean;
  }>(),
  { pending: false, detail: false },
);

const trend = computed(() => priceTrend(props.entries));
</script>

<template>
  <EnvelopeGraph v-if="trend.hasEnoughHistory" :data="trend.points" :detail="detail" />
  <div
    v-else
    class="text-2xs flex h-full items-center justify-center font-mono tracking-wide text-slate-400 uppercase dark:text-slate-500"
  >
    {{ pending ? 'Chargement…' : "Pas encore assez d'historique" }}
  </div>
</template>
