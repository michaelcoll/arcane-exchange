<script setup lang="ts">
import {
  BINDERS,
  DEFAULT_RARITY_RULES,
  MAX_KEPT_COPIES,
  RARITY_DISTRIBUTION,
  TOTAL_COPIES,
  binderFactor,
  copiesOf,
  eligibleCopies,
  fmtInt,
  uniqueOf,
  type RarityRule,
  type TradeRuleRarity,
} from '~/utils/trade-rules';

const rules = reactive<Record<TradeRuleRarity, RarityRule>>({ ...DEFAULT_RARITY_RULES });
const binders = ref<string[]>(['trade', 'bulk']);

const toggleBinder = (key: string) => {
  binders.value = binders.value.includes(key)
    ? binders.value.filter((k) => k !== key)
    : [...binders.value, key];
};

const setKeep = (code: TradeRuleRarity, value: number) => {
  rules[code].keep = Math.min(MAX_KEPT_COPIES, Math.max(0, value));
};

const rows = computed(() =>
  RARITY_DISTRIBUTION.map((r) => {
    const rule = rules[r.code];
    const copies = copiesOf(r);
    const proposed = rule.on
      ? eligibleCopies(r, rule.keep, binderFactor(r.code, binders.value))
      : 0;
    return {
      rarity: r,
      rule,
      unique: uniqueOf(r),
      copies,
      proposed,
      kept: rule.on ? copies - proposed : 0,
      excluded: rule.on ? 0 : copies,
    };
  }),
);

const totals = computed(() => ({
  proposed: rows.value.reduce((s, x) => s + x.proposed, 0),
  kept: rows.value.reduce((s, x) => s + x.kept, 0),
  excluded: rows.value.reduce((s, x) => s + x.excluded, 0),
}));

const pct = (n: number) => (n / TOTAL_COPIES) * 100;

const RARITY_INK: Record<TradeRuleRarity, string> = {
  M: 'text-slate-800 dark:text-slate-100',
  R: 'text-slate-600 dark:text-slate-300',
  U: 'text-slate-500 dark:text-slate-400',
  C: 'text-slate-400 dark:text-slate-500',
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
          <span class="text-xs text-slate-400 dark:text-slate-500"
            >issus de ton dernier import</span
          >
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="b in BINDERS"
            :key="b.key"
            :class="[
              'inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-solid px-3 py-1.5 text-xs font-medium whitespace-nowrap transition-all duration-150 select-none',
              binders.includes(b.key)
                ? 'border-cyan-500/40 bg-cyan-500/10 text-cyan-700 dark:border-cyan-400/40 dark:bg-cyan-400/10 dark:text-cyan-300'
                : 'border-slate-200 bg-slate-100 text-slate-600 hover:border-slate-300 hover:bg-slate-200 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100',
            ]"
            @click="toggleBinder(b.key)"
          >
            <Icon :name="binders.includes(b.key) ? 'lucide:check' : 'lucide:plus'" size="12" />
            {{ b.name }}
            <span class="font-mono text-xs opacity-60">{{ fmtInt(b.cards) }}</span>
          </button>
        </div>
        <span class="text-xs text-slate-400 dark:text-slate-500">
          Seules les cartes rangées dans un binder coché peuvent partir. Laisser « Decks » décoché
          suffit à protéger tes listes montées.
        </span>
      </div>
    </div>

    <!-- MATRICE DES RARETÉS -->
    <div
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
        :key="row.rarity.code"
        :class="[
          'grid grid-cols-[1fr_58px_132px_104px] items-center gap-2.5 rounded-xl px-3.5 py-2.5 transition-all duration-150 hover:bg-slate-100/70 dark:hover:bg-white/5',
          'max-sm:mb-2 max-sm:grid-cols-1 max-sm:gap-0 max-sm:border max-sm:border-slate-200 max-sm:bg-white max-sm:px-3 max-sm:pt-3 max-sm:pb-2.5 max-sm:hover:bg-white dark:max-sm:border-white/10 dark:max-sm:bg-zinc-900 dark:max-sm:hover:bg-zinc-900',
          row.rule.on ? '' : 'opacity-45',
        ]"
      >
        <div class="flex items-center gap-3 max-sm:mt-1.5">
          <span
            :class="[
              'grid h-7 w-7 shrink-0 place-items-center rounded-lg border border-slate-300 bg-slate-100 font-mono text-xs font-bold dark:border-white/15 dark:bg-zinc-800',
              RARITY_INK[row.rarity.code],
            ]"
            >{{ row.rarity.code }}</span
          >
          <div class="flex flex-col gap-px">
            <span class="text-sm font-semibold text-slate-800 dark:text-slate-100">{{
              row.rarity.label
            }}</span>
            <span class="text-xs text-slate-400 dark:text-slate-500">
              {{ fmtInt(row.unique) }} cartes · {{ fmtInt(row.copies) }} ex.
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
            :aria-pressed="row.rule.on"
            :aria-label="`Ouvrir la rareté ${row.rarity.label} à l'échange`"
            :class="[
              'relative h-7 w-12 shrink-0 cursor-pointer rounded-full transition-all duration-200',
              row.rule.on
                ? 'border border-transparent bg-cyan-500 dark:bg-cyan-400'
                : 'border border-slate-300 bg-slate-200 dark:border-white/15 dark:bg-zinc-800',
            ]"
            @click="row.rule.on = !row.rule.on"
          >
            <span
              :class="[
                'absolute top-1 h-5 w-5 rounded-full transition-[left] duration-200 ease-out',
                row.rule.on ? 'left-6 bg-zinc-950' : 'left-1 bg-slate-500 dark:bg-slate-400',
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
              :disabled="row.rule.keep <= 0"
              aria-label="Moins"
              @click="setKeep(row.rarity.code, row.rule.keep - 1)"
            >
              −
            </button>
            <b
              class="min-w-[38px] text-center font-mono text-xs font-semibold text-slate-800 dark:text-slate-100"
              >{{ row.rule.keep }}</b
            >
            <button
              :class="stepperBtnClass"
              :disabled="row.rule.keep >= MAX_KEPT_COPIES"
              aria-label="Plus"
              @click="setKeep(row.rarity.code, row.rule.keep + 1)"
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
