<script setup lang="ts">
import type { RarityCode } from '~/bindings/RarityCode';
import { MAX_KEPT_COPIES, fmtInt } from '~/utils/trade-rules';
import { RARITY_LABELS } from '~/utils/rarity';

const { getCollectionStats, getRarityFilters, setRarityFilter } = useCollectionService();
const { getTradeBinders, addTradeBinder, removeTradeBinder } = useUserService();
const { showError } = useToast();

const errorMessage = (e: unknown) =>
  (e as { data?: { error?: string } })?.data?.error ?? 'Une erreur est survenue.';

// useAsyncData must be called synchronously during setup, not inside onMounted.
const { data: statsData, pending: statsPending, error: statsError } = getCollectionStats();
const binders = computed(() => statsData.value?.binders ?? []);

const {
  data: filtersData,
  pending: filtersPending,
  error: filtersError,
  refresh: refreshFilters,
} = getRarityFilters();
const rows = computed(() => filtersData.value?.rarities ?? []);

watch(statsError, (e) => {
  if (e) showError('Impossible de charger les binders', errorMessage(e));
});

watch(filtersError, (e) => {
  if (e) showError('Impossible de charger tes règles de rareté', errorMessage(e));
});

const selectedBinders = ref<string[]>([]);
const selectionLoading = ref(true);
const bindersLoading = computed(() => statsPending.value || selectionLoading.value);
const binderBusy = ref<string | null>(null);
const rarityBusy = ref<string | null>(null);

onMounted(async () => {
  try {
    const tradeBinders = await getTradeBinders();
    selectedBinders.value = tradeBinders.binders;
  } catch (e) {
    showError('Impossible de charger ta sélection de binders', errorMessage(e));
  } finally {
    selectionLoading.value = false;
  }
});

const toggleBinder = async (name: string) => {
  const previous = selectedBinders.value;
  const wasSelected = previous.includes(name);
  selectedBinders.value = wasSelected ? previous.filter((n) => n !== name) : [...previous, name];

  binderBusy.value = name;
  try {
    if (wasSelected) {
      await removeTradeBinder(name);
    } else {
      await addTradeBinder(name);
    }
    await refreshFilters();
  } catch (e) {
    selectedBinders.value = previous;
    showError('Impossible de mettre à jour le binder', errorMessage(e));
  } finally {
    binderBusy.value = null;
  }
};

const toggleRarity = async (rarity: string, isOpen: boolean, keptCopies: number) => {
  rarityBusy.value = rarity;
  try {
    await setRarityFilter(rarity, !isOpen, keptCopies);
  } catch (e) {
    showError('Impossible de mettre à jour la rareté', errorMessage(e));
  } finally {
    await refreshFilters();
    rarityBusy.value = null;
  }
};

const setKeep = async (rarity: string, isOpen: boolean, value: number) => {
  const keep = Math.min(MAX_KEPT_COPIES, Math.max(0, value));
  rarityBusy.value = rarity;
  try {
    await setRarityFilter(rarity, isOpen, keep);
  } catch (e) {
    showError('Impossible de mettre à jour la rareté', errorMessage(e));
  } finally {
    await refreshFilters();
    rarityBusy.value = null;
  }
};

const totals = computed(() => ({
  proposed: rows.value.reduce((s, r) => s + r.proposed, 0),
  kept: rows.value.reduce((s, r) => s + (r.is_open ? r.copies - r.proposed : 0), 0),
  excluded: rows.value.reduce((s, r) => s + (r.is_open ? 0 : r.copies), 0),
}));

const totalCopies = computed(() => rows.value.reduce((s, r) => s + r.copies, 0));

const pct = (n: number) => (totalCopies.value === 0 ? 0 : (n / totalCopies.value) * 100);

const RARITY_INK: Record<RarityCode, string> = {
  M: 'text-slate-800 dark:text-slate-100',
  R: 'text-slate-600 dark:text-slate-300',
  U: 'text-slate-500 dark:text-slate-400',
  C: 'text-slate-400 dark:text-slate-500',
  S: 'text-slate-400 dark:text-slate-500',
};

const labelClass =
  'text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500';
const stepperBtnClass =
  'grid h-[26px] w-[26px] cursor-pointer place-items-center rounded-md text-base leading-none text-slate-500 transition-all duration-150 hover:bg-slate-200 hover:text-slate-800 disabled:cursor-default disabled:opacity-30 disabled:hover:bg-transparent dark:text-slate-400 dark:hover:bg-zinc-700 dark:hover:text-slate-100 dark:disabled:hover:bg-transparent';
</script>

<template>
  <section class="mb-6">
    <div class="mb-3 flex flex-col gap-1.5">
      <span :class="labelClass">Ce que je propose à l'échange</span>
      <span class="text-xs text-slate-400 dark:text-slate-500">
        Une ligne par rareté : tu ouvres, et tu fixes le nombre d'exemplaires que tu gardes
        toujours.
      </span>
    </div>

    <!-- PÉRIMÈTRE · BINDERS -->
    <div
      class="mb-3 rounded-2xl border border-slate-200 bg-white/60 px-4 py-3.5 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
    >
      <div class="flex flex-col gap-2.5">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <span :class="labelClass">Périmètre · binders ManaBox</span>
          <span class="flex items-center gap-1.5 text-xs text-slate-400 dark:text-slate-500">
            <Icon
              v-if="bindersLoading"
              name="lucide:loader-circle"
              size="14"
              class="animate-spin"
            />
            issus de ton dernier import
          </span>
        </div>
        <div
          v-if="!bindersLoading && binders.length === 0"
          class="text-xs text-slate-400 dark:text-slate-500"
        >
          Aucun binder dans ton dernier import ManaBox.
        </div>
        <div v-else class="flex flex-wrap gap-2">
          <button
            v-for="b in binders"
            :key="b.name"
            :disabled="binderBusy === b.name"
            :class="[
              'inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-solid px-3 py-1.5 text-xs font-medium whitespace-nowrap transition-all duration-150 select-none disabled:cursor-default disabled:opacity-60',
              selectedBinders.includes(b.name)
                ? 'border-cyan-500/40 bg-cyan-500/10 text-cyan-700 dark:border-cyan-400/40 dark:bg-cyan-400/10 dark:text-cyan-300'
                : 'border-slate-200 bg-slate-100 text-slate-600 hover:border-slate-300 hover:bg-slate-200 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100',
            ]"
            @click="toggleBinder(b.name)"
          >
            <Icon
              :name="selectedBinders.includes(b.name) ? 'lucide:check' : 'lucide:plus'"
              size="12"
            />
            {{ b.name }}
            <span class="font-mono text-xs opacity-60">{{ fmtInt(b.card_count) }}</span>
          </button>
        </div>
        <span class="text-xs text-slate-400 dark:text-slate-500">
          Seules les cartes rangées dans un binder coché peuvent partir.
        </span>
      </div>
    </div>

    <!-- ÉTAT VIDE -->
    <div
      v-if="!filtersPending && rows.length === 0"
      class="rounded-2xl border border-slate-200 bg-white/60 px-4 py-6 text-center text-xs text-slate-400 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60 dark:text-slate-500"
    >
      Coche au moins un binder ci-dessus pour voir apparaître les raretés que tu peux proposer à
      l'échange.
    </div>

    <!-- MATRICE DES RARETÉS -->
    <div
      v-else
      class="rounded-2xl border border-slate-200 bg-white/60 px-1.5 pt-3.5 pb-1.5 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
    >
      <div
        class="grid grid-cols-[1fr_58px_132px_104px] items-center gap-2.5 px-3.5 pb-2.5 max-sm:hidden"
      >
        <span :class="labelClass">Rareté</span>
        <span :class="labelClass">Ouverte</span>
        <span :class="labelClass">Copies gardées</span>
        <span :class="[labelClass, 'text-right']">Proposés</span>
      </div>

      <div
        v-for="row in rows"
        :key="row.rarity"
        :class="[
          'grid grid-cols-[1fr_58px_132px_104px] items-center gap-2.5 rounded-xl px-3.5 py-2.5 transition-all duration-150 hover:bg-slate-100/70 dark:hover:bg-white/5',
          'max-sm:mb-2 max-sm:grid-cols-1 max-sm:gap-0 max-sm:border max-sm:border-slate-200 max-sm:bg-white max-sm:px-3 max-sm:pt-3 max-sm:pb-2.5 max-sm:hover:bg-white dark:max-sm:border-white/10 dark:max-sm:bg-zinc-900 dark:max-sm:hover:bg-zinc-900',
          row.is_open ? '' : 'opacity-45',
        ]"
      >
        <div class="flex items-center gap-3 max-sm:mt-1.5">
          <span
            :class="[
              'grid h-7 w-7 shrink-0 place-items-center rounded-lg border border-slate-300 bg-slate-100 font-mono text-xs font-bold dark:border-white/15 dark:bg-zinc-800',
              RARITY_INK[row.rarity as RarityCode],
            ]"
            >{{ row.rarity }}</span
          >
          <div class="flex flex-col gap-px">
            <span class="text-sm font-semibold text-slate-800 dark:text-slate-100">{{
              RARITY_LABELS[row.rarity as RarityCode]
            }}</span>
            <span class="text-xs text-slate-400 dark:text-slate-500">
              {{ fmtInt(row.copies) }} ex.
            </span>
          </div>
        </div>

        <div
          class="flex items-center max-sm:mt-2.5 max-sm:justify-between max-sm:gap-3.5 max-sm:border-t max-sm:border-slate-200 max-sm:pt-2.5 dark:max-sm:border-white/5"
        >
          <span
            class="hidden text-xs tracking-wide text-slate-400 uppercase max-sm:block dark:text-slate-500"
            >Ouverte</span
          >
          <button
            :aria-pressed="row.is_open"
            :aria-label="`Ouvrir la rareté ${RARITY_LABELS[row.rarity as RarityCode]} à l'échange`"
            :disabled="rarityBusy === row.rarity"
            :class="[
              'relative h-7 w-12 shrink-0 cursor-pointer rounded-full transition-all duration-200 disabled:cursor-default disabled:opacity-60',
              row.is_open
                ? 'border border-transparent bg-cyan-500 dark:bg-cyan-400'
                : 'border border-slate-300 bg-slate-200 dark:border-white/15 dark:bg-zinc-800',
            ]"
            @click="toggleRarity(row.rarity, row.is_open, row.kept_copies)"
          >
            <span
              :class="[
                'absolute top-1 h-5 w-5 rounded-full transition-[left] duration-200 ease-out',
                row.is_open ? 'left-6 bg-zinc-950' : 'left-1 bg-slate-500 dark:bg-slate-400',
              ]"
            />
          </button>
        </div>

        <div
          class="flex items-center max-sm:mt-2.5 max-sm:justify-between max-sm:gap-3.5 max-sm:border-t max-sm:border-slate-200 max-sm:pt-2.5 dark:max-sm:border-white/5"
        >
          <span
            class="hidden text-xs tracking-wide text-slate-400 uppercase max-sm:block dark:text-slate-500"
            >Copies gardées</span
          >
          <div
            class="inline-flex items-center gap-0.5 rounded-xl border border-slate-200 bg-slate-100/70 p-1 dark:border-white/10 dark:bg-black/20"
          >
            <button
              :class="stepperBtnClass"
              :disabled="rarityBusy === row.rarity || row.kept_copies <= 0"
              aria-label="Moins"
              @click="setKeep(row.rarity, row.is_open, row.kept_copies - 1)"
            >
              −
            </button>
            <b
              class="min-w-[38px] text-center font-mono text-xs font-semibold text-slate-800 dark:text-slate-100"
              >{{ row.kept_copies }}</b
            >
            <button
              :class="stepperBtnClass"
              :disabled="rarityBusy === row.rarity || row.kept_copies >= MAX_KEPT_COPIES"
              aria-label="Plus"
              @click="setKeep(row.rarity, row.is_open, row.kept_copies + 1)"
            >
              +
            </button>
          </div>
        </div>

        <div
          class="flex items-center justify-end max-sm:mt-2.5 max-sm:justify-between max-sm:gap-3.5 max-sm:border-t max-sm:border-slate-200 max-sm:pt-2.5 dark:max-sm:border-white/5"
        >
          <span
            class="hidden text-xs tracking-wide text-slate-400 uppercase max-sm:block dark:text-slate-500"
            >Proposés</span
          >
          <span
            :class="[
              'font-mono text-sm font-bold tracking-tight whitespace-nowrap',
              row.proposed
                ? 'text-cyan-600 dark:text-cyan-400'
                : 'text-slate-400 dark:text-slate-500',
            ]"
            >{{ fmtInt(row.proposed) }}</span
          >
        </div>
      </div>

      <div class="mx-3.5 my-1.5 h-px bg-slate-200 dark:bg-white/10" />

      <!-- RÉPARTITION -->
      <div class="flex flex-col gap-2.5 px-3.5 pt-2 pb-3.5">
        <div
          class="flex h-2.5 overflow-hidden rounded-full border border-slate-200 bg-slate-200/70 dark:border-white/5 dark:bg-zinc-800"
        >
          <i
            class="block bg-cyan-500 transition-[width] duration-300 dark:bg-cyan-400"
            :style="{ width: pct(totals.proposed) + '%' }"
          />
          <i
            class="block bg-slate-400 transition-[width] duration-300 dark:bg-slate-600"
            :style="{ width: pct(totals.kept) + '%' }"
          />
          <i
            class="block bg-slate-300 transition-[width] duration-300 dark:bg-zinc-700"
            :style="{ width: pct(totals.excluded) + '%' }"
          />
        </div>
        <div class="flex flex-wrap gap-x-4 gap-y-2 text-xs text-slate-400 dark:text-slate-500">
          <span class="inline-flex items-center gap-1.5">
            <i class="block h-2 w-2 rounded-[2px] bg-cyan-500 dark:bg-cyan-400" />
            <span>Proposés {{ fmtInt(totals.proposed) }}</span>
          </span>
          <span class="inline-flex items-center gap-1.5">
            <i class="block h-2 w-2 rounded-[2px] bg-slate-400 dark:bg-slate-600" />
            <span>Gardés par tes règles {{ fmtInt(totals.kept) }}</span>
          </span>
          <span class="inline-flex items-center gap-1.5">
            <i class="block h-2 w-2 rounded-[2px] bg-slate-300 dark:bg-zinc-700" />
            <span>Raretés fermées {{ fmtInt(totals.excluded) }}</span>
          </span>
        </div>
      </div>
    </div>
  </section>
</template>
