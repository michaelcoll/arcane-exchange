<script setup lang="ts">
definePageMeta({ middleware: 'auth' });

const mode = ref<'eur' | 'edh'>('eur');

const give = [
  { name: 'Black Market Connections', color: 'b', eur: 13, edh: 18 },
  { name: 'Reprieve', color: 'w', eur: 4, edh: 7 },
  { name: 'Persistent Constrictor', color: 'b', eur: 2, edh: 12 },
];
const get = [
  { name: 'Sire of Seven Deaths', color: 'm', eur: 31, edh: 21 },
  { name: 'The Soul Stone', color: 'b', eur: 9, edh: 5 },
];

const giveTotal = computed(() =>
  give.reduce((s, c) => s + (mode.value === 'eur' ? c.eur : c.edh), 0),
);
const getTotal = computed(() =>
  get.reduce((s, c) => s + (mode.value === 'eur' ? c.eur : c.edh), 0),
);
const diff = computed(() => getTotal.value - giveTotal.value);

const even = computed(() => Math.abs(diff.value) < (mode.value === 'eur' ? 3 : 2));
const lean = computed(() => (even.value ? 'even' : diff.value > 0 ? 'lean-me' : 'lean-them'));
const verdict = computed(() => {
  if (even.value) return mode.value === 'eur' ? 'Équilibré' : '≈ équilibré';
  const abs = Math.abs(diff.value);
  if (diff.value > 0) return mode.value === 'eur' ? `+€${abs} pour toi` : `+${abs} pts pour toi`;
  return mode.value === 'eur' ? `+€${abs} pour lui` : `+${abs} pts pour lui`;
});
const getShare = computed(() => {
  const total = giveTotal.value + getTotal.value || 1;
  return (getTotal.value / total) * 100;
});

const fmt = (v: number) => (mode.value === 'eur' ? `€${v}` : `${v}%`);

const giveEurSum = give.reduce((s, c) => s + c.eur, 0);
const getEurSum = get.reduce((s, c) => s + c.eur, 0);

const modeOptions = [
  { value: 'eur', label: 'Prix €', tone: 'cyan' },
  { value: 'edh', label: 'EDHREC %', tone: 'vio' },
];
</script>

<template>
  <div class="mx-auto max-w-[1180px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <!-- HEADER -->
    <div class="mb-5 flex flex-wrap items-center justify-between gap-3.5">
      <div class="flex items-center gap-3">
        <button
          class="grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
        >
          <Icon name="lucide:chevron-left" size="16" />
        </button>
        <div class="flex items-center gap-2.5">
          <PlayerAvatar initials="M4" :online="true" />
          <div class="flex flex-col gap-px">
            <h2 class="font-display text-base font-semibold tracking-tight">
              Échange avec <span class="text-cyan-600 dark:text-cyan-400">@mizzix_42</span>
            </h2>
            <span class="text-xs text-slate-400 dark:text-slate-500">
              <Icon
                name="mdi:star"
                size="11"
                class="align-[-1px] text-violet-500 dark:text-violet-300"
              />
              4,8 · 2,3 km · en ligne
            </span>
          </div>
        </div>
      </div>
      <SegToggle v-model="mode" :options="modeOptions" />
    </div>

    <!-- EDHREC INFO BANNER -->
    <div
      v-if="mode === 'edh'"
      class="mb-4 flex items-start gap-2.5 rounded-2xl border border-violet-500/30 bg-violet-500/10 px-4 py-3 shadow-lg backdrop-blur-md dark:border-violet-400/30 dark:bg-violet-400/10"
    >
      <span class="mt-px text-violet-500 dark:text-violet-300"
        ><Icon name="lucide:info" size="16"
      /></span>
      <span class="text-sm text-slate-600 dark:text-slate-300">
        Le <span class="text-violet-500 dark:text-violet-300">% EDHREC</span> = part des decks
        référencés qui jouent la carte. On compare la désirabilité plutôt que l'euro.
      </span>
    </div>

    <!-- TRADE GRID -->
    <div
      class="grid [grid-template-columns:1fr_auto_1fr] items-stretch gap-4 max-md:[grid-template-columns:1fr]"
    >
      <!-- LEFT: Je donne -->
      <div
        class="flex flex-col gap-3 rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="flex items-center justify-between gap-4">
          <span
            class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
            >Je donne</span
          >
          <span class="text-xs text-slate-400 dark:text-slate-500">{{ give.length }} cartes</span>
        </div>
        <div class="flex flex-col gap-2">
          <div
            v-for="(c, i) in give"
            :key="i"
            class="flex items-center gap-3 rounded-xl border border-slate-200 bg-white px-3 py-2 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 dark:border-white/10 dark:bg-zinc-900 dark:hover:border-white/15 dark:hover:bg-zinc-800"
          >
            <MtgCard :name="c.name" :mini="true" class="w-7 flex-none" />
            <div class="min-w-0 flex-1">
              <div
                class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
              >
                {{ c.name }}
              </div>
            </div>
            <span
              :class="[
                'font-mono text-sm',
                mode === 'eur'
                  ? 'text-slate-600 dark:text-slate-300'
                  : 'text-violet-500 dark:text-violet-300',
              ]"
            >
              {{ mode === 'eur' ? `€${c.eur}` : `${c.edh}%` }}
            </span>
            <button
              class="grid h-7 w-7 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
            >
              <Icon name="lucide:x" size="13" />
            </button>
          </div>
        </div>
        <button
          class="flex items-center justify-center gap-2 rounded-xl border-[1.5px] border-dashed border-slate-300 bg-black/10 p-3 text-sm font-semibold text-slate-600 transition-all duration-200 hover:border-cyan-500/40 hover:bg-cyan-500/10 dark:border-white/15 dark:text-slate-300 dark:hover:border-cyan-400/40 dark:hover:bg-cyan-400/10"
        >
          <Icon name="lucide:plus" size="16" /> Ajouter une de mes cartes
        </button>
        <div class="mt-auto h-px bg-slate-200 dark:bg-white/10" />
        <div class="flex items-center justify-between gap-4">
          <span class="text-sm text-slate-400 dark:text-slate-500">{{
            mode === 'eur' ? 'Total' : 'Cumul inclusion'
          }}</span>
          <span
            :class="[
              'font-mono text-xl font-bold tracking-tight whitespace-nowrap',
              mode === 'edh' ? 'text-violet-500 dark:text-violet-300' : '',
            ]"
          >
            {{ fmt(giveTotal) }}
          </span>
        </div>
      </div>

      <!-- CENTER: Balance split -->
      <div class="flex min-w-[168px] flex-col items-center justify-center gap-3.5">
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
              fmt(giveTotal)
            }}</span>
            <span class="font-mono text-sm font-semibold text-cyan-600 dark:text-cyan-400">{{
              fmt(getTotal)
            }}</span>
          </div>
          <div
            :class="[
              'rounded-xl border px-3 py-2 text-center font-mono text-sm font-semibold',
              lean === 'even'
                ? 'border-cyan-500/30 bg-cyan-500/10 text-cyan-700 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300'
                : 'border-violet-500/30 bg-violet-500/10 text-violet-700 dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300',
            ]"
          >
            {{ verdict }}
          </div>
        </div>
        <button
          v-if="mode === 'edh'"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-violet-500/30 bg-violet-500/10 px-3 py-1.5 text-xs font-medium whitespace-nowrap text-violet-700 transition-all duration-150 select-none dark:border-violet-400/30 dark:bg-violet-400/10 dark:text-violet-300"
        >
          <Icon name="lucide:refresh-cw" size="12" />
          Voir aussi en € · €{{ giveEurSum }} ↔ €{{ getEurSum }}
        </button>
      </div>

      <!-- RIGHT: Je reçois -->
      <div
        class="flex flex-col gap-3 rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="flex items-center justify-between gap-4">
          <span
            class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
            >Je reçois</span
          >
          <span class="text-xs text-slate-400 dark:text-slate-500">{{ get.length }} cartes</span>
        </div>
        <div class="flex flex-col gap-2">
          <div
            v-for="(c, i) in get"
            :key="i"
            class="flex items-center gap-3 rounded-xl border border-slate-200 bg-white px-3 py-2 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 dark:border-white/10 dark:bg-zinc-900 dark:hover:border-white/15 dark:hover:bg-zinc-800"
          >
            <MtgCard :name="c.name" :mini="true" class="w-7 flex-none" />
            <div class="min-w-0 flex-1">
              <div
                class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
              >
                {{ c.name }}
              </div>
            </div>
            <span
              :class="[
                'font-mono text-sm',
                mode === 'eur'
                  ? 'text-cyan-600 dark:text-cyan-400'
                  : 'text-violet-500 dark:text-violet-300',
              ]"
            >
              {{ mode === 'eur' ? `€${c.eur}` : `${c.edh}%` }}
            </span>
            <button
              class="grid h-7 w-7 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
            >
              <Icon name="lucide:x" size="13" />
            </button>
          </div>
        </div>
        <button
          class="flex items-center justify-center gap-2 rounded-xl border-[1.5px] border-dashed border-slate-300 bg-black/10 p-3 text-sm font-semibold text-slate-600 transition-all duration-200 hover:border-cyan-500/40 hover:bg-cyan-500/10 dark:border-white/15 dark:text-slate-300 dark:hover:border-cyan-400/40 dark:hover:bg-cyan-400/10"
        >
          <Icon name="lucide:plus" size="16" /> Chercher dans sa collection
        </button>
        <div class="mt-auto h-px bg-slate-200 dark:bg-white/10" />
        <div class="flex items-center justify-between gap-4">
          <span class="text-sm text-slate-400 dark:text-slate-500">{{
            mode === 'eur' ? 'Total' : 'Cumul inclusion'
          }}</span>
          <span
            class="font-mono text-xl font-bold tracking-tight whitespace-nowrap text-cyan-600 dark:text-cyan-400"
            >{{ fmt(getTotal) }}</span
          >
        </div>
      </div>
    </div>

    <!-- ACTIONS -->
    <div
      class="mt-5 rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
    >
      <div class="flex flex-wrap items-center justify-between gap-3.5">
        <div class="flex flex-col gap-0.5">
          <span
            class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
            >Contact via le canal du joueur</span
          >
          <span class="text-xs text-slate-400 dark:text-slate-500"
            >Aucun paiement ni messagerie internes · redirection externe</span
          >
        </div>
        <div class="flex flex-wrap items-center gap-2.5">
          <button
            class="inline-flex items-center justify-center gap-2 rounded-xl border border-slate-200 bg-transparent px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
          >
            Enregistrer le brouillon
          </button>
          <button
            class="inline-flex items-center justify-center gap-2 rounded-xl border border-slate-300 bg-slate-100 px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap text-slate-800 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-200 active:translate-y-0 dark:border-white/15 dark:bg-zinc-800 dark:text-slate-100 dark:hover:border-white/15 dark:hover:bg-zinc-700"
          >
            Proposer l'échange
          </button>
          <button
            class="inline-flex items-center justify-center gap-2 rounded-xl border border-transparent bg-cyan-500 px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-cyan-400 active:translate-y-0 dark:bg-cyan-400 dark:hover:bg-cyan-300"
          >
            <Icon name="simple-icons:discord" size="16" /> Contacter sur Discord
            <Icon name="lucide:arrow-up-right" size="14" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
