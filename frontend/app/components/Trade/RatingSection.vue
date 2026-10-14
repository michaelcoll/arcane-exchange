<script setup lang="ts">
import type { TradeViewRating } from '~/composables/useTradeView';

/* Rating under the sides (mobile and desktop), like the iOS `ratingSection`. It shows the Trade
 * view's rating, which says whether I can still rate; the page only mounts it when it is visible.
 * A 0 rating means "rating skipped", not "zero stars". */
defineProps<{
  partner: string;
  rating: TradeViewRating;
  busy: boolean;
}>();

const emit = defineEmits<{ rate: [value: number]; skip: [] }>();
</script>

<template>
  <section
    class="flex flex-col gap-2.5 rounded-2xl bg-slate-400/10 p-[14px]"
    aria-labelledby="trade-rating-title"
  >
    <h3 id="trade-rating-title" class="text-sm font-semibold">
      Noter <span class="text-secondary">@{{ partner }}</span>
    </h3>

    <template v-if="rating.canRate">
      <TradeRatingStars :value="null" :read-only="busy" @rate="emit('rate', $event)" />
      <button
        type="button"
        class="w-fit text-[13px] font-medium text-[var(--ink-2)] underline-offset-2 hover:underline disabled:opacity-50"
        :disabled="busy"
        @click="emit('skip')"
      >
        Passer la notation
      </button>
    </template>
    <span
      v-else-if="rating.mine != null"
      data-part="mine"
      class="text-secondary inline-flex items-center gap-1.5 text-[13px] font-medium"
    >
      <Icon :name="rating.mine === 0 ? 'lucide:circle-minus' : 'mdi:star'" :size="14" />
      {{ rating.mine === 0 ? 'Notation passée' : `Tu as mis ${rating.mine}/5` }}
    </span>

    <p data-part="caption" class="text-xs text-[var(--ink-3)]">
      <template v-if="rating.partner == null">
        Optionnel. L'échange se clôture dès que vous avez tous les deux noté ou passé.
      </template>
      <template v-else>
        <span class="text-secondary">@{{ partner }}</span>
        {{ rating.partner === 0 ? 'a passé la notation.' : `t'a mis ${rating.partner}/5.` }}
      </template>
    </p>
  </section>
</template>
