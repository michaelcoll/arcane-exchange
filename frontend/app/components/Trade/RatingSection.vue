<script setup lang="ts">
import type { TradeRating } from '~/utils/trade';

/* Notation sous les rails (mobile), en COMPLETED et CLOSED, comme la `ratingSection` côté iOS.
 * Une note 0 veut dire « notation passée », pas « zéro étoile ». */
defineProps<{
  partner: string;
  meRating: TradeRating;
  partnerRating: TradeRating;
  busy: boolean;
}>();

const emit = defineEmits<{ rate: [value: number] }>();
</script>

<template>
  <section
    class="flex flex-col gap-2.5 rounded-2xl bg-slate-400/10 p-[14px]"
    aria-labelledby="trade-rating-title"
  >
    <h3 id="trade-rating-title" class="text-sm font-semibold">
      Noter <span class="text-secondary">@{{ partner }}</span>
    </h3>

    <span
      v-if="meRating != null"
      data-part="mine"
      class="text-secondary inline-flex items-center gap-1.5 text-[13px] font-medium"
    >
      <Icon :name="meRating === 0 ? 'lucide:circle-minus' : 'mdi:star'" :size="14" />
      {{ meRating === 0 ? 'Notation passée' : `Tu as mis ${meRating}/5` }}
    </span>
    <template v-else>
      <TradeRatingStars :value="null" :read-only="busy" @rate="emit('rate', $event)" />
      <button
        type="button"
        class="w-fit text-[13px] font-medium text-[var(--ink-2)] underline-offset-2 hover:underline disabled:opacity-50"
        :disabled="busy"
        @click="emit('rate', 0)"
      >
        Passer la notation
      </button>
    </template>

    <p data-part="caption" class="text-xs text-[var(--ink-3)]">
      <template v-if="partnerRating == null">
        Optionnel. L'échange se clôture dès que vous avez tous les deux noté ou passé.
      </template>
      <template v-else>
        <span class="text-secondary">@{{ partner }}</span>
        {{ partnerRating === 0 ? 'a passé la notation.' : `t'a mis ${partnerRating}/5.` }}
      </template>
    </p>
  </section>
</template>
