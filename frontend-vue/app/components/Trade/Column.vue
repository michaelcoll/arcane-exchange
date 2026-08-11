<script setup lang="ts">
import type { TradeCard } from '~/bindings/TradeCard';

const props = defineProps<{
  label: string;
  cards: TradeCard[];
  /** `cyan` pour la colonne « Je reçois », `neutral` pour « Je donne ». */
  accent: 'cyan' | 'neutral';
  /** Les cartes de l'échange sont réservées. */
  reserved: boolean;
  /** Cette colonne autorise le retrait d'une carte (uniquement « Je reçois »). */
  removable: boolean;
  /** Bouton d'ajout affiché seulement si fourni. */
  addLabel?: string;
}>();

const emit = defineEmits<{ remove: [card: TradeCard]; add: [] }>();

const cardKey = (c: TradeCard) =>
  `${c.set_code}-${c.collector_number}-${c.language_code}-${c.foil}`;

const totalQuantity = computed(() => props.cards.reduce((s, c) => s + c.quantity, 0));

const total = computed(() => tradeCardsTotal(props.cards));

const valueClasses = computed(() =>
  props.accent === 'cyan'
    ? 'text-cyan-600 dark:text-cyan-400'
    : 'text-slate-600 dark:text-slate-300',
);

const totalClasses = computed(() =>
  props.accent === 'cyan' ? 'text-cyan-600 dark:text-cyan-400' : '',
);
</script>

<template>
  <div
    class="flex min-w-[240px] flex-col gap-3 rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
  >
    <div class="flex items-center justify-between gap-4">
      <span
        class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >{{ label }}</span
      >
      <span class="text-xs text-slate-400 dark:text-slate-500"
        >{{ totalQuantity }} carte{{ totalQuantity > 1 ? 's' : '' }}</span
      >
    </div>

    <div class="flex flex-col gap-2">
      <div
        v-for="c in cards"
        :key="cardKey(c)"
        :class="[
          'flex items-center gap-3 rounded-xl border px-3 py-2 transition-all duration-150',
          reserved
            ? 'border-violet-500/30 bg-violet-500/[0.07] dark:border-violet-400/40 dark:bg-violet-400/[0.07]'
            : 'border-slate-200 bg-white hover:border-slate-300 hover:bg-slate-50 dark:border-white/10 dark:bg-zinc-900 dark:hover:border-white/15 dark:hover:bg-zinc-800',
        ]"
      >
        <MtgCard
          :name="c.name"
          :scryfall-id="c.scryfall_id"
          :the-gatherer-id="c.the_gatherer_id ?? undefined"
          :mini="true"
          class="w-7 flex-none"
        />
        <div class="min-w-0 flex-1">
          <div
            class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
          >
            {{ c.name }}
            <span v-if="c.quantity > 1" class="font-mono text-xs text-slate-400 dark:text-slate-500"
              >×{{ c.quantity }}</span
            >
          </div>
          <div
            v-if="reserved"
            class="flex items-center gap-1 text-[11px] text-violet-500 dark:text-violet-300"
          >
            <Icon name="lucide:lock" size="10" /> Réservée
          </div>
        </div>
        <span :class="['font-mono text-sm', valueClasses]">
          {{ formatPrice(tradeCardValue(c)) }}
        </span>
        <button
          v-if="removable"
          class="grid h-[26px] w-[26px] flex-none place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
          aria-label="Retirer la carte de l’échange"
          @click="emit('remove', c)"
        >
          <Icon name="lucide:x" size="13" />
        </button>
        <span
          v-else-if="reserved"
          class="grid h-[26px] w-[26px] flex-none place-items-center rounded-lg border border-slate-200 bg-slate-100 text-violet-500 opacity-50 dark:border-white/10 dark:bg-white/5 dark:text-violet-300"
          aria-hidden="true"
        >
          <Icon name="lucide:lock" size="12" />
        </span>
      </div>

      <div v-if="!cards.length" class="px-0.5 py-2 text-xs text-slate-400 dark:text-slate-500">
        Aucune carte de ce côté.
      </div>
    </div>

    <button
      v-if="addLabel"
      class="flex items-center justify-center gap-2 rounded-xl border-[1.5px] border-dashed border-slate-300 bg-black/10 p-3 text-sm font-semibold text-slate-600 transition-all duration-200 hover:border-cyan-500/40 hover:bg-cyan-500/10 dark:border-white/15 dark:text-slate-300 dark:hover:border-cyan-400/40 dark:hover:bg-cyan-400/10"
      @click="emit('add')"
    >
      <Icon name="lucide:plus" size="16" /> {{ addLabel }}
    </button>

    <div class="mt-auto h-px bg-slate-200 dark:bg-white/10" />
    <div class="flex items-center justify-between gap-4">
      <span class="text-sm text-slate-400 dark:text-slate-500">Total</span>
      <span
        :class="['font-mono text-xl font-bold tracking-tight whitespace-nowrap', totalClasses]"
        >{{ formatPrice(total) }}</span
      >
    </div>
  </div>
</template>
