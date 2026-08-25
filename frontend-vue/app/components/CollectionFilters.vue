<script setup lang="ts">
import type { RarityCode } from '~/bindings/RarityCode';
import type { SetInfo } from '~/bindings/SetInfo';
import { RARITY_LABELS, RARITY_ORDER } from '~/utils/rarity';
import { resolveSetName } from '~/utils/set';

const props = withDefaults(
  defineProps<{
    active: { rar: RarityCode[]; sets: string[] };
    setList: SetInfo[];
    priceMin?: number;
    priceMax?: number;
    showSearch?: boolean;
  }>(),
  {
    showSearch: true,
  },
);

const emit = defineEmits<{
  toggle: [k: 'rar' | 'sets', v: string];
  'price-change': [lo: number, hi: number];
}>();

const q = defineModel<string>('q', { default: '' });

const sliderMax = computed(() => props.priceMax ?? 150);

const lo = ref(props.priceMin ?? 0);
const hi = ref(props.priceMax ?? 150);

watch(
  () => props.priceMin,
  (v) => {
    if (v != null) lo.value = v;
  },
  { immediate: true },
);
watch(
  () => props.priceMax,
  (v) => {
    if (v != null) hi.value = v;
  },
  { immediate: true },
);

const pricePct = (v: number) => (v / sliderMax.value) * 100;

const onLoInput = (e: Event) => {
  const val = Number((e.target as HTMLInputElement).value);
  lo.value = Math.min(val, hi.value - 1);
  emit('price-change', lo.value, hi.value);
};
const onHiInput = (e: Event) => {
  const val = Number((e.target as HTMLInputElement).value);
  hi.value = Math.max(val, lo.value + 1);
  emit('price-change', lo.value, hi.value);
};

const chipClass = (r: RarityCode) =>
  props.active.rar.includes(r)
    ? 'text-cyan-700 dark:text-cyan-300 border-cyan-500/30 dark:border-cyan-400/30 bg-cyan-500/10 dark:bg-cyan-400/10'
    : 'text-slate-600 dark:text-slate-300 bg-slate-100 dark:bg-white/5 border-slate-200 dark:border-white/10 hover:text-slate-800 dark:hover:text-slate-100 hover:border-slate-300 dark:hover:border-white/15 hover:bg-slate-50 dark:hover:bg-zinc-800';

/* ── Combobox set selector ── */
const cbxOpen = ref(false);
const cbxQuery = ref('');
const cbxEl = ref<HTMLElement | null>(null);

onClickOutside(cbxEl, () => {
  cbxOpen.value = false;
});

const filteredSets = computed(() => {
  const qLower = cbxQuery.value.toLowerCase();
  if (!qLower) return props.setList;
  return props.setList.filter(
    (s) => s.name.toLowerCase().includes(qLower) || s.code.toLowerCase().includes(qLower),
  );
});

const toggleCbx = () => {
  cbxOpen.value = !cbxOpen.value;
  if (!cbxOpen.value) cbxQuery.value = '';
};

const clearSets = () => {
  [...props.active.sets].forEach((code) => emit('toggle', 'sets', code));
};
</script>

<template>
  <div class="flex h-full flex-col gap-4">
    <!-- Search -->
    <div
      v-if="showSearch"
      class="flex items-center gap-2.5 rounded-xl border border-slate-400/50 bg-slate-200/50 px-3 py-2 transition-all duration-200 focus-within:border-cyan-500/40 focus-within:bg-slate-200 focus-within:ring-4 focus-within:ring-cyan-500/10 dark:border-white/15 dark:bg-black/20 dark:focus-within:border-cyan-400/40 dark:focus-within:bg-black/30"
    >
      <Icon
        name="lucide:search"
        :size="16"
        class="shrink-0 text-slate-500/80 dark:text-slate-500"
      />
      <input
        v-model="q"
        placeholder="Filtrer ma collection…"
        class="min-w-0 flex-1 border-0 bg-transparent text-sm text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
      />
    </div>

    <!-- Rarity -->
    <div class="flex flex-col gap-2">
      <span
        class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Rareté</span
      >
      <div class="flex flex-wrap gap-2">
        <button
          v-for="r in RARITY_ORDER"
          :key="r"
          :class="[
            'inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-solid px-3 py-1.5 text-xs font-medium whitespace-nowrap transition-all duration-150 select-none',
            chipClass(r),
          ]"
          @click="emit('toggle', 'rar', r)"
        >
          {{ RARITY_LABELS[r] }}
        </button>
      </div>
    </div>

    <!-- Price range -->
    <div class="flex flex-col gap-2">
      <span
        class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Plage de prix</span
      >
      <div
        class="rounded-xl border border-slate-400/50 bg-slate-200/50 px-3.5 pt-4 pb-3 dark:border-white/15 dark:bg-black/20"
      >
        <div class="relative h-5">
          <div
            class="absolute top-1/2 right-0 left-0 h-1 -translate-y-1/2 rounded-full bg-slate-300 dark:bg-zinc-700"
          />
          <div
            class="absolute top-1/2 h-1 -translate-y-1/2 rounded-full bg-cyan-500 dark:bg-cyan-400"
            :style="{ left: pricePct(lo) + '%', right: 100 - pricePct(hi) + '%' }"
          />
          <input
            type="range"
            min="0"
            :max="sliderMax"
            :value="lo"
            :style="{ zIndex: lo > sliderMax * 0.92 ? 5 : 3 }"
            class="z-[3]"
            aria-label="Prix minimum"
            @input="onLoInput"
          />
          <input
            type="range"
            min="0"
            :max="sliderMax"
            :value="hi"
            class="z-[4]"
            aria-label="Prix maximum"
            @input="onHiInput"
          />
        </div>
        <div class="mt-3 flex justify-between gap-2">
          <span
            class="rounded-md border border-cyan-500/30 bg-cyan-500/10 px-2.5 py-0.5 text-xs text-cyan-700 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300"
            >{{ lo }} €</span
          >
          <span class="font-mono text-xs text-slate-400 dark:text-slate-500">—</span>
          <span
            class="rounded-md border border-cyan-500/30 bg-cyan-500/10 px-2.5 py-0.5 text-xs text-cyan-700 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300"
            >{{ hi }}{{ hi >= sliderMax ? '+' : '' }} €</span
          >
        </div>
      </div>
    </div>

    <!-- Set / Extension — combobox -->
    <div class="flex flex-col gap-2">
      <span
        class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Set / Extension</span
      >
      <div ref="cbxEl" class="relative">
        <!-- Trigger -->
        <button
          type="button"
          :class="[
            'flex min-h-[42px] w-full cursor-pointer items-center gap-2 rounded-xl border border-solid border-slate-400/50 bg-slate-200/50 py-2 pr-2.5 pl-3 text-left transition-[border-color,box-shadow,background] duration-150 hover:bg-slate-200 dark:border-white/15 dark:bg-black/20 dark:hover:bg-black/30',
            cbxOpen ? 'border-cyan-500/40 ring-4 ring-cyan-500/10 dark:border-cyan-400/40' : '',
          ]"
          aria-haspopup="listbox"
          :aria-expanded="cbxOpen"
          @click="toggleCbx"
        >
          <span
            v-if="active.sets.length === 0"
            class="flex-1 text-sm text-slate-400 dark:text-slate-500"
            >Toutes les extensions</span
          >
          <span v-else class="flex min-w-0 flex-1 flex-wrap gap-1.5">
            <span
              v-for="code in active.sets"
              :key="code"
              class="inline-flex max-w-full items-center gap-1.5 rounded-lg border border-cyan-500/30 bg-cyan-500/10 py-0.5 pr-1 pl-2 text-xs font-medium text-cyan-700 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300"
            >
              <i :class="['ss', 'text-sm', `ss-${code.toLowerCase()}`]" />
              <span class="max-w-[120px] overflow-hidden text-ellipsis whitespace-nowrap">{{
                resolveSetName(props.setList, code)
              }}</span>
              <i
                class="grid h-4 w-4 flex-none cursor-pointer place-items-center rounded text-cyan-700 opacity-70 hover:bg-cyan-500/20 hover:opacity-100 dark:text-cyan-300"
                role="button"
                :aria-label="`Retirer ${resolveSetName(props.setList, code)}`"
                @click.stop="emit('toggle', 'sets', code)"
              >
                <Icon name="lucide:x" :size="11" />
              </i>
            </span>
          </span>
          <span
            :class="[
              'inline-flex flex-none text-slate-400 transition-transform duration-200 dark:text-slate-500',
              cbxOpen ? 'rotate-180' : '',
            ]"
          >
            <Icon name="lucide:chevron-down" :size="16" />
          </span>
        </button>

        <!-- Popover -->
        <Transition name="cbx-pop">
          <div
            v-if="cbxOpen"
            class="absolute top-[calc(100%+8px)] right-0 left-0 z-40 rounded-xl border border-slate-300 bg-white p-2.5 shadow-2xl dark:border-white/15 dark:bg-zinc-800"
            role="listbox"
            aria-multiselectable="true"
          >
            <!-- Search inside popup -->
            <div
              class="flex items-center gap-2 rounded-lg border border-slate-200 bg-slate-100 px-3 py-2 dark:border-white/10 dark:bg-black/20"
            >
              <Icon
                name="lucide:search"
                :size="15"
                class="shrink-0 text-slate-400 dark:text-slate-500"
              />
              <input
                v-model="cbxQuery"
                placeholder="Filtrer une extension…"
                class="min-w-0 flex-1 border-0 bg-transparent text-sm text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
              />
            </div>

            <!-- Options list -->
            <div
              class="mt-2 flex max-h-60 flex-col gap-0.5 overflow-x-hidden overflow-y-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
            >
              <div
                v-if="filteredSets.length === 0"
                class="px-1 py-4 text-center text-xs text-slate-300 dark:text-slate-600"
              >
                Aucune extension
              </div>
              <button
                v-for="set in filteredSets"
                :key="set.code"
                type="button"
                :class="[
                  'group grid w-full cursor-pointer grid-cols-[24px_1fr] items-center gap-2.5 rounded-lg px-2.5 py-2 text-left transition-colors duration-100',
                  active.sets.includes(set.code)
                    ? 'on bg-cyan-500/10 dark:bg-cyan-400/10'
                    : 'hover:bg-slate-100 dark:hover:bg-zinc-700',
                ]"
                role="option"
                :aria-selected="active.sets.includes(set.code)"
                @click="emit('toggle', 'sets', set.code)"
              >
                <span
                  class="grid place-items-center text-slate-600 group-[.on]:text-cyan-700 dark:text-slate-300 dark:group-[.on]:text-cyan-300"
                >
                  <i :class="['ss', 'text-lg', `ss-${set.code.toLowerCase()}`]" />
                </span>
                <span
                  class="min-w-0 overflow-hidden text-sm text-ellipsis whitespace-nowrap text-slate-800 group-[.on]:font-medium group-[.on]:text-cyan-700 dark:text-slate-100 dark:group-[.on]:text-cyan-300"
                  >{{ set.name }}</span
                >
              </button>
            </div>

            <!-- Clear all -->
            <button
              v-if="active.sets.length > 0"
              type="button"
              class="mt-2 w-full cursor-pointer rounded-lg border border-slate-200 bg-black/10 p-2 font-mono text-xs font-semibold tracking-wide text-slate-400 transition-all duration-150 hover:border-cyan-500/30 hover:bg-cyan-500/10 hover:text-cyan-700 dark:border-white/10 dark:text-slate-500 dark:hover:border-cyan-400/30 dark:hover:bg-cyan-400/10 dark:hover:text-cyan-300"
              @click="clearSets"
            >
              Tout effacer ({{ active.sets.length }})
            </button>
          </div>
        </Transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Range inputs */
input[type='range'] {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  width: 100%;
  height: 20px;
  margin: 0;
  -webkit-appearance: none;
  appearance: none;
  background: none;
  pointer-events: none;
}

input[type='range']::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  pointer-events: auto;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  cursor: grab;
  background: radial-gradient(circle at 35% 30%, #fff, var(--cyan));
  border: 1px solid var(--cyan);
  box-shadow:
    0 0 0 3px rgba(34, 211, 238, 0.18),
    0 2px 6px rgba(0, 0, 0, 0.5);
  transition: box-shadow 0.15s;
}

input[type='range']::-webkit-slider-thumb:active {
  cursor: grabbing;
  box-shadow:
    0 0 0 5px rgba(34, 211, 238, 0.18),
    0 2px 6px rgba(0, 0, 0, 0.5);
}

input[type='range']::-moz-range-thumb {
  pointer-events: auto;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  cursor: grab;
  background: radial-gradient(circle at 35% 30%, #fff, var(--cyan));
  border: 1px solid var(--cyan);
  box-shadow:
    0 0 0 3px rgba(34, 211, 238, 0.18),
    0 2px 6px rgba(0, 0, 0, 0.5);
}

/* Popover transition */
.cbx-pop-enter-active,
.cbx-pop-leave-active {
  transition:
    opacity 0.14s ease,
    transform 0.14s ease;
}

.cbx-pop-enter-from,
.cbx-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
