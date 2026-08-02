<script setup lang="ts">
// Sélecteur de joueur — champ de recherche + panneau enrichi (avatar, nb cartes,
// nb échanges, note). Données mockées en attendant l'API de recherche de joueurs.
interface Player {
  handle: string;
  initials: string;
  cards: number;
  trades: number;
  rating: string;
  recent?: boolean;
}

const MOCK_PLAYERS: Player[] = [
  { handle: 'mizzix_42', initials: 'M4', cards: 412, trades: 23, rating: '4,8', recent: true },
  { handle: 'golgari_jo', initials: 'GO', cards: 1284, trades: 41, rating: '4,9', recent: true },
  { handle: 'kaalia_dt', initials: 'KA', cards: 876, trades: 12, rating: '5,0' },
  { handle: 'urza_main', initials: 'UR', cards: 233, trades: 8, rating: '4,4' },
  { handle: 'sheoldred_x', initials: 'SX', cards: 640, trades: 17, rating: '4,7' },
  { handle: 'simic_ramp', initials: 'SI', cards: 97, trades: 3, rating: '4,1' },
];

const props = defineProps<{
  modelValue: Player | null;
  cta?: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: Player | null];
  submit: [value: Player];
}>();

const query = ref('');
const open = ref(false);
const highlighted = ref(0);
const navActive = ref(false);
const wrapRef = ref<HTMLElement | null>(null);
const listRef = ref<HTMLElement | null>(null);

const filtered = computed(() => {
  const q = query.value.trim().replace(/^@/, '').toLowerCase();
  return MOCK_PLAYERS.filter((p) => p.handle.toLowerCase().includes(q));
});
const recents = computed(() => filtered.value.filter((p) => p.recent));
const others = computed(() => filtered.value.filter((p) => !p.recent));
const ordered = computed(() => [...recents.value, ...others.value]);

const onClickOutside = (e: MouseEvent) => {
  if (wrapRef.value && !wrapRef.value.contains(e.target as Node)) open.value = false;
};

onMounted(() => document.addEventListener('mousedown', onClickOutside));
onUnmounted(() => document.removeEventListener('mousedown', onClickOutside));

const select = (p: Player, submit = false) => {
  open.value = false;
  query.value = '';
  highlighted.value = 0;
  navActive.value = false;
  emit('update:modelValue', p);
  if (submit) emit('submit', p);
};

const clear = () => {
  emit('update:modelValue', null);
  query.value = '';
  open.value = true;
};

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    open.value = false;
    navActive.value = false;
    return;
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    open.value = true;
    if (!navActive.value) {
      navActive.value = true;
      highlighted.value = 0;
    } else {
      highlighted.value = Math.min(highlighted.value + 1, ordered.value.length - 1);
    }
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    navActive.value = true;
    highlighted.value = Math.max(highlighted.value - 1, 0);
  } else if (e.key === 'Enter' && open.value) {
    const target = ordered.value.length === 1 ? ordered.value[0] : ordered.value[highlighted.value];
    if (target) {
      e.preventDefault();
      select(target, true);
    }
  }
};

watch([highlighted, open, navActive], () => {
  if (!open.value || !navActive.value) return;
  nextTick(() => {
    const box = listRef.value;
    if (!box) return;
    const row = box.querySelectorAll('[data-prow]')[highlighted.value] as HTMLElement | undefined;
    if (!row) return;
    const pad = 8;
    const top = row.offsetTop - pad;
    const bottom = row.offsetTop + row.offsetHeight + pad;
    if (top < box.scrollTop) box.scrollTop = top;
    else if (bottom > box.scrollTop + box.clientHeight) box.scrollTop = bottom - box.clientHeight;
  });
});
</script>

<template>
  <div ref="wrapRef" class="relative w-full">
    <!-- Selected state -->
    <div
      v-if="props.modelValue"
      class="flex min-h-[62px] items-center gap-2.5 rounded-2xl border border-violet-500/40 bg-violet-500/10 py-2 pr-2 pl-3 dark:border-violet-400/40 dark:bg-violet-400/10"
    >
      <PlayerAvatar :initials="props.modelValue.initials" />
      <span class="flex min-w-0 flex-1 flex-col">
        <span class="truncate text-sm font-semibold text-slate-800 dark:text-slate-100"
          ><span class="text-slate-400 dark:text-slate-500">@</span
          >{{ props.modelValue.handle }}</span
        >
        <span class="font-mono text-xs text-violet-600 dark:text-violet-300"
          >{{ props.modelValue.cards }} cartes</span
        >
      </span>
      <button
        class="grid h-8 w-8 flex-none place-items-center rounded-lg text-slate-400 transition-colors duration-150 hover:bg-black/10 hover:text-slate-600 dark:text-slate-500 dark:hover:bg-white/10 dark:hover:text-slate-300"
        aria-label="Changer de joueur"
        @click="clear"
      >
        <AppIcon name="x" :size="14" />
      </button>
      <button
        class="inline-flex items-center justify-center gap-2 self-stretch rounded-xl border border-transparent bg-violet-500 px-6 text-base leading-none font-bold whitespace-nowrap text-zinc-950 shadow-lg transition-all duration-150 hover:-translate-y-px hover:bg-violet-400 active:translate-y-0 dark:bg-violet-400 dark:hover:bg-violet-300"
        @click="emit('submit', props.modelValue)"
      >
        {{ cta ?? 'Voir ses cartes' }}
      </button>
    </div>

    <!-- Search state -->
    <div
      v-else
      class="flex min-h-[62px] items-center gap-2.5 rounded-2xl border border-solid border-violet-400/40 bg-slate-200/75 py-2 pr-3 pl-4 transition-all duration-200 focus-within:border-violet-500/50 focus-within:ring-4 focus-within:ring-violet-500/10 dark:border-violet-400/25 dark:bg-black/20 dark:focus-within:border-violet-400/50"
    >
      <AppIcon name="user" :size="16" class="shrink-0 text-violet-500/80 dark:text-violet-300/80" />
      <input
        v-model="query"
        class="min-w-0 flex-1 border-0 bg-transparent text-base text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
        placeholder="Cherche un joueur : @pseudo…"
        @focus="open = true"
        @input="
          open = true;
          highlighted = 0;
          navActive = false;
        "
        @keydown="onKeydown"
      />
      <span
        class="text-2xs rounded-md border border-slate-300 px-1.5 py-0.5 font-mono text-slate-400 dark:border-white/15 dark:text-slate-500"
        >⏎</span
      >
    </div>

    <!-- Dropdown -->
    <div
      v-if="open && !props.modelValue"
      class="absolute inset-x-0 top-[calc(100%+8px)] z-20 overflow-hidden rounded-2xl border border-slate-200 bg-white/95 shadow-xl backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/95"
    >
      <div ref="listRef" class="max-h-[280px] overflow-y-auto p-1.5" @mousemove="navActive = false">
        <div
          v-if="recents.length > 0"
          class="text-2xs px-2.5 pt-2 pb-1 font-mono tracking-widest text-slate-400 uppercase dark:text-slate-500"
        >
          récents
        </div>
        <button
          v-for="(p, i) in recents"
          :key="p.handle"
          data-prow
          class="flex w-full items-center gap-2.5 rounded-xl px-2.5 py-2 text-left transition-colors duration-100"
          :class="
            navActive && ordered[highlighted] === p
              ? 'bg-violet-500/10 dark:bg-violet-400/10'
              : 'hover:bg-slate-100 dark:hover:bg-white/5'
          "
          @mousedown.prevent
          @click="select(p, false)"
        >
          <PlayerAvatar :initials="p.initials" />
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="truncate text-sm font-semibold text-slate-800 dark:text-slate-100"
              ><span class="text-slate-400 dark:text-slate-500">@</span>{{ p.handle }}</span
            >
            <span class="truncate text-xs text-slate-400 dark:text-slate-500"
              >{{ p.cards }} cartes · {{ p.trades }} échanges</span
            >
          </span>
          <span
            class="inline-flex flex-none items-center gap-1 font-mono text-xs text-slate-400 dark:text-slate-500"
          >
            <AppIcon name="star" :size="12" />{{ p.rating }}
          </span>
        </button>

        <div
          v-if="others.length > 0"
          class="text-2xs px-2.5 pt-2 pb-1 font-mono tracking-widest text-slate-400 uppercase dark:text-slate-500"
        >
          autres joueurs
        </div>
        <button
          v-for="p in others"
          :key="p.handle"
          data-prow
          class="flex w-full items-center gap-2.5 rounded-xl px-2.5 py-2 text-left transition-colors duration-100"
          :class="
            navActive && ordered[highlighted] === p
              ? 'bg-violet-500/10 dark:bg-violet-400/10'
              : 'hover:bg-slate-100 dark:hover:bg-white/5'
          "
          @mousedown.prevent
          @click="select(p, false)"
        >
          <PlayerAvatar :initials="p.initials" />
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="truncate text-sm font-semibold text-slate-800 dark:text-slate-100"
              ><span class="text-slate-400 dark:text-slate-500">@</span>{{ p.handle }}</span
            >
            <span class="truncate text-xs text-slate-400 dark:text-slate-500"
              >{{ p.cards }} cartes · {{ p.trades }} échanges</span
            >
          </span>
          <span
            class="inline-flex flex-none items-center gap-1 font-mono text-xs text-slate-400 dark:text-slate-500"
          >
            <AppIcon name="star" :size="12" />{{ p.rating }}
          </span>
        </button>

        <div
          v-if="!filtered.length"
          class="px-2.5 py-4 text-center text-sm text-slate-400 dark:text-slate-500"
        >
          Aucun joueur ne correspond à « {{ query }} »
        </div>
      </div>
      <div
        class="flex items-center justify-between gap-3 border-t border-slate-200 px-3 py-2 dark:border-white/10"
      >
        <span
          class="text-2xs flex items-center gap-1.5 font-mono text-slate-400 dark:text-slate-500"
        >
          <span class="rounded border border-slate-300 px-1 dark:border-white/15">↑</span>
          <span class="rounded border border-slate-300 px-1 dark:border-white/15">↓</span> naviguer
          <span class="rounded border border-slate-300 px-1 dark:border-white/15">⏎</span> ouvrir sa
          collection
        </span>
        <span class="font-mono text-xs text-violet-600 dark:text-violet-300"
          >{{ filtered.length }} joueurs</span
        >
      </div>
    </div>
  </div>
</template>
