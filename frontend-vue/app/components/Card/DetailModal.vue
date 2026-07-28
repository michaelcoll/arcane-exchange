<script setup lang="ts">
import type { CardOffer } from '~/bindings/CardOffer';
import type { CollectionCard } from '~/bindings/CollectionCard';
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';

const props = defineProps<{
  card: CollectionCard;
}>();

defineEmits<{
  close: [];
}>();

const { getCardPriceHistory, getCardOffers } = useCardsService();

const cardHistoryData = ref<PriceHistoryEntry[]>([]);
const cardHistoryPending = ref(false);

const cardOffers = ref<CardOffer[]>([]);
const cardOffersTotal = ref(0);
const cardOffersPending = ref(false);

watch(
  () => props.card,
  async (card) => {
    cardHistoryData.value = [];
    cardOffers.value = [];
    cardOffersTotal.value = 0;
    cardHistoryPending.value = true;
    cardOffersPending.value = true;
    try {
      const [history, offers] = await Promise.all([
        getCardPriceHistory(card.scryfall_id),
        getCardOffers({
          set_code: card.set_code,
          collector_number: card.collector_number,
          language_code: card.language_code,
          foil: card.foil,
          sort_by: 'selling_price',
          page: 0,
          page_size: 6,
        }),
      ]);
      cardHistoryData.value = history;
      cardOffers.value = offers.items;
      cardOffersTotal.value = offers.total;
    } finally {
      cardHistoryPending.value = false;
      cardOffersPending.value = false;
    }
  },
  { immediate: true },
);

const initialsOf = (username: string) => username.replace(/^@/, '').slice(0, 2).toUpperCase();

const cardEnvelopeData = computed(() => toEnvelopeData(cardHistoryData.value));
const cardHasEnoughHistory = computed(() => cardEnvelopeData.value.length >= 2);
const cardVariation = computed(() => computeVariation(cardHistoryData.value));
</script>

<template>
  <div
    class="fixed inset-0 z-[80] grid animate-[fade_0.2s_ease] place-items-center bg-black/60 px-5 pt-[calc(1.25rem+env(safe-area-inset-top))] pb-5 backdrop-blur-sm"
    @click="$emit('close')"
  >
    <div
      class="relative max-h-[calc(100dvh-40px-env(safe-area-inset-top))] w-full max-w-[840px] animate-[pop_0.26s_cubic-bezier(0.3,1.2,0.4,1)] overflow-hidden rounded-3xl border border-slate-300 p-0 shadow-2xl max-[720px]:max-w-[440px] dark:border-white/15"
      @click.stop
    >
      <!-- close -->
      <button
        class="absolute top-3.5 right-3.5 z-[5] grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
        @click="$emit('close')"
      >
        <Icon name="lucide:x" :size="16" />
      </button>

      <!-- body grid -->
      <div
        class="grid max-h-[calc(100dvh-40px-env(safe-area-inset-top))] [grid-template-columns:minmax(300px,360px)_1fr] overflow-y-auto max-[720px]:[grid-template-columns:1fr]"
      >
        <!-- art -->
        <div
          class="relative flex items-center justify-center border-r border-slate-200/60 bg-white/40 p-7 backdrop-blur-md max-[720px]:border-r-0 max-[720px]:border-b max-[720px]:p-6 dark:border-white/10 dark:bg-zinc-900/40"
        >
          <MtgCard
            :scryfall-id="card.scryfall_id"
            :the-gatherer-id="card.the_gatherer_id ?? undefined"
            :name="card.name"
            class="w-full max-w-[300px] drop-shadow-2xl max-[720px]:max-w-[260px]"
          />
        </div>

        <!-- info -->
        <div class="flex min-w-0 flex-col gap-4 bg-white px-6 py-7 dark:bg-zinc-900">
          <!-- header -->
          <div>
            <h3 class="font-display mb-1.5 text-xl font-semibold tracking-tight">
              {{ card.name }}
            </h3>
            <span
              class="inline-flex flex-wrap items-center gap-2 text-sm text-slate-400 dark:text-slate-500"
            >
              {{ card.set_code.toUpperCase() }} · {{ card.rarity_code }}
              <span
                v-if="card.foil"
                class="text-2xs ml-2 inline-flex [animation:foilSlide_4s_linear_infinite] items-center rounded-full [background-size:200%_100%] px-1.5 py-px font-bold tracking-wide text-zinc-900 [background:linear-gradient(110deg,#ffd84d,#4dffd0,#4db4ff,#b85dff,#ff5db8)]"
              >
                ✦ Foil
              </span>
            </span>
          </div>

          <!-- stats -->
          <div class="grid grid-cols-3 gap-2.5">
            <div
              class="flex flex-col gap-1 rounded-xl border border-slate-200 bg-white px-3 py-3 dark:border-white/10 dark:bg-zinc-900"
            >
              <span
                class="text-2xs font-mono tracking-widest text-slate-400 uppercase dark:text-slate-500"
                >Quantité</span
              >
              <span class="font-mono text-lg font-bold tracking-tight"
                >×{{ card.collection_entry?.quantity ?? 0 }}</span
              >
            </div>
            <div
              class="flex flex-col gap-1 rounded-xl border border-slate-200 bg-white px-3 py-3 dark:border-white/10 dark:bg-zinc-900"
            >
              <span
                class="text-2xs font-mono tracking-widest text-slate-400 uppercase dark:text-slate-500"
                >Prix unit.</span
              >
              <span class="font-mono text-lg font-bold tracking-tight">{{
                formatPrice(card.collection_entry?.purchase_price ?? 0)
              }}</span>
            </div>
            <div
              class="flex flex-col gap-1 rounded-xl border border-slate-200 bg-white px-3 py-3 dark:border-white/10 dark:bg-zinc-900"
            >
              <span
                class="text-2xs font-mono tracking-widest text-slate-400 uppercase dark:text-slate-500"
                >Total</span
              >
              <span
                class="font-mono text-lg font-bold tracking-tight text-cyan-600 dark:text-cyan-400"
                >{{
                  formatPrice(
                    (card.collection_entry?.quantity ?? 0) *
                      (card.collection_entry?.purchase_price ?? 0),
                  )
                }}</span
              >
            </div>
          </div>

          <!-- market -->
          <div
            class="rounded-xl border border-slate-200 bg-black/20 px-3.5 py-3 dark:border-white/10"
          >
            <div class="flex items-center justify-between">
              <span
                class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
                >Marché · CardMarket · 30 j</span
              >
              <span
                :class="[
                  'font-mono text-xs',
                  cardVariation.positive
                    ? 'text-cyan-600 dark:text-cyan-400'
                    : 'text-red-500 dark:text-red-400',
                ]"
              >
                {{ cardVariation.positive ? '▴' : '▾' }}
                {{ Math.abs(cardVariation.pct).toFixed(0) }} %
              </span>
            </div>
            <div class="mt-2 flex items-center gap-2.5">
              <span class="font-mono text-xl font-bold">{{
                formatPrice(card.price_guide?.trend ?? 0)
              }}</span>
            </div>
            <div class="mt-2 h-[140px]">
              <EnvelopeGraph v-if="cardHasEnoughHistory" :data="cardEnvelopeData" detail />
              <div
                v-else
                class="text-2xs flex h-full items-center justify-center font-mono tracking-wide text-slate-400 uppercase dark:text-slate-500"
              >
                {{ cardHistoryPending ? 'Chargement…' : "Pas encore assez d'historique" }}
              </div>
            </div>
          </div>

          <!-- sellers -->
          <div class="mt-auto flex flex-col gap-2.5">
            <span class="text-sm text-slate-400 dark:text-slate-500">
              <template v-if="cardOffersPending">Chargement…</template>
              <template v-else-if="cardOffersTotal > 0">
                <b class="font-semibold text-slate-800 dark:text-slate-100"
                  >{{ cardOffersTotal }} joueur{{ cardOffersTotal > 1 ? 's' : '' }}</b
                >
                possède{{ cardOffersTotal > 1 ? 'nt' : '' }} cette carte
              </template>
              <template v-else>Personne d'autre ne possède cette carte pour l'instant.</template>
            </span>

            <div
              v-if="cardOffers.length > 0"
              class="grid grid-cols-2 gap-2 max-[720px]:grid-cols-1"
            >
              <div
                v-for="(offer, i) in cardOffers"
                :key="offer.owner_username"
                :class="[
                  'relative flex flex-col gap-2.5 rounded-xl border px-3.5 py-3',
                  i === 0
                    ? 'border-cyan-500/30 bg-cyan-500/5 dark:border-cyan-400/30 dark:bg-cyan-400/5'
                    : 'border-slate-200 bg-white dark:border-white/10 dark:bg-zinc-900',
                ]"
              >
                <span
                  v-if="i === 0"
                  class="text-2xs absolute -top-2.5 left-3 rounded-full bg-cyan-500 px-2 py-0.5 font-semibold tracking-wide whitespace-nowrap text-zinc-950 dark:bg-cyan-400"
                >
                  Meilleure offre
                </span>
                <div class="flex items-center gap-2.5">
                  <PlayerAvatar :initials="initialsOf(offer.owner_username)" />
                  <div class="min-w-0 flex-1">
                    <div
                      class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
                    >
                      {{ offer.owner_username }}
                    </div>
                    <span class="text-xs text-slate-400 dark:text-slate-500"
                      >×{{ offer.quantity }} dispo</span
                    >
                  </div>
                </div>
                <div class="flex items-center justify-between">
                  <span class="font-mono text-base font-bold tracking-tight">{{
                    formatPrice(offer.selling_price ?? 0)
                  }}</span>
                  <NuxtLink
                    to="/trade"
                    class="inline-flex items-center justify-center gap-2 rounded-lg border border-transparent bg-cyan-500 px-3 py-1.5 text-xs leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
                    >Échanger</NuxtLink
                  >
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
