<script setup lang="ts">
const props = defineProps<{
  scryfallId?: string;
  theGathererId?: string;
  qty?: number;
  clickable?: boolean;
  mini?: boolean;
  foil?: boolean;
  name?: string;
  size?: 'sm' | 'md' | 'lg';
  reserved?: boolean;
}>();

const emit = defineEmits(['click']);

const src = computed(
  () =>
    (props.theGathererId &&
      `https://gatherer-static.wizards.com/Cards/medium/${props.theGathererId}.webp`) ||
    (props.scryfallId &&
      `https://api.scryfall.com/cards/${props.scryfallId}?format=image&version=normal`) ||
    '',
);

const cardRef = ref<HTMLElement | null>(null);

let raf: number | null = null;

const updateFoil = () => {
  const el = cardRef.value;
  if (!el || !props.foil) return;
  const r = el.getBoundingClientRect();
  const vh = window.innerHeight || 1;
  const p = (r.top + r.height / 2) / vh;
  el.style.setProperty('--foil', Math.max(-0.5, Math.min(1.5, p)).toFixed(4));
};

const onScroll = () => {
  if (!raf)
    raf = requestAnimationFrame(() => {
      raf = null;
      updateFoil();
    });
};

onMounted(() => {
  if (!props.foil) return;
  updateFoil();
  window.addEventListener('scroll', onScroll, { passive: true, capture: true });
  window.addEventListener('resize', onScroll);
});

onUnmounted(() => {
  window.removeEventListener('scroll', onScroll, { capture: true });
  window.removeEventListener('resize', onScroll);
  if (raf) cancelAnimationFrame(raf);
});
</script>

<template>
  <div
    ref="cardRef"
    :class="[
      'relative flex aspect-[5/7] flex-col gap-0 overflow-hidden',
      'ease transition-[transform,box-shadow,border-color] duration-200',
      'border',
      'rounded-[4%]',
      src
        ? 'border-black/55 bg-zinc-950 p-0'
        : [
            mini ? 'p-0.5' : 'p-[5%]',
            'border-slate-300 bg-slate-100 shadow-lg dark:border-white/15 dark:bg-zinc-800',
          ],
      clickable
        ? 'cursor-pointer hover:-translate-y-1 hover:border-cyan-500/30 hover:shadow-xl dark:hover:border-cyan-400/30'
        : '',
      foil ? 'foil' : '',
      reserved ? 'ring-2 ring-violet-500 dark:ring-violet-400' : '',
    ]"
    :title="name"
    @click="emit('click')"
  >
    <!-- qty badge -->
    <span
      v-if="qty != null"
      class="absolute top-1.5 right-1.5 z-[5] rounded-full border border-white/20 bg-black/60 px-1.5 py-0.5 font-mono text-xs font-semibold text-zinc-100 backdrop-blur-sm"
      >×{{ qty }}</span
    >

    <!-- reserved badge -->
    <span
      v-if="reserved && !mini"
      class="text-2xs absolute top-1.5 left-1.5 z-[5] inline-flex items-center gap-1 rounded-full border border-violet-400/40 bg-violet-500/20 px-1.5 py-0.5 font-mono font-bold text-violet-100 backdrop-blur-sm"
    >
      <Icon name="lucide:lock" :size="10" />
      Réservée
    </span>

    <!-- inner vignette overlay — replaces ::after pseudo-element -->
    <div
      v-if="!src"
      :class="[
        'pointer-events-none absolute inset-0 z-[1]',
        mini
          ? 'rounded shadow-[inset_0_0_0_2px_rgba(0,0,0,0.55)]'
          : 'rounded-md shadow-[inset_0_0_0_4px_rgba(0,0,0,0.55)]',
      ]"
    />

    <!-- real card scan -->
    <template v-if="src">
      <img
        class="absolute inset-0 z-0 block h-full w-full object-cover select-none"
        style="border-radius: inherit"
        :src="src"
        :alt="name ?? ''"
        loading="lazy"
        draggable="false"
      />
    </template>

    <!-- mini placeholder -->
    <template v-else-if="mini">
      <div
        class="relative z-[2] grid flex-1 place-items-center overflow-hidden rounded border border-slate-200 bg-slate-100 dark:border-white/10 dark:bg-zinc-800"
      >
        <AppIcon
          name="mountain"
          class="text-slate-300 opacity-50 dark:text-slate-600"
          style="width: 46%; height: 46%"
        />
      </div>
    </template>

    <!-- full placeholder -->
    <template v-else>
      <!-- title bar -->
      <div
        class="relative z-[2] flex items-center justify-between gap-1.5 rounded border border-slate-200 bg-slate-200 px-[6%] py-[5%] dark:border-white/10 dark:bg-zinc-800"
      >
        <span class="h-1.5 w-[60%] rounded-sm bg-slate-300 dark:bg-slate-600" />
        <span
          class="h-3 w-3 shrink-0 rounded-full bg-zinc-700 shadow-[inset_0_1px_1px_rgba(255,255,255,0.25)]"
        />
      </div>
      <!-- art -->
      <div
        class="relative z-[2] mt-[5%] grid flex-1 place-items-center overflow-hidden rounded border border-slate-200 bg-slate-100 dark:border-white/10 dark:bg-zinc-800"
      >
        <AppIcon
          name="mountain"
          :size="28"
          class="text-slate-300 opacity-50 dark:text-slate-600"
          style="width: 34%; height: 34%"
        />
      </div>
      <!-- type bar -->
      <div
        class="relative z-[2] mt-[5%] flex items-center gap-1.5 rounded border border-slate-200 bg-slate-200 px-[6%] py-[5%] dark:border-white/10 dark:bg-zinc-800"
      >
        <span class="h-1.5 w-[50%] rounded-sm bg-slate-300 opacity-70 dark:bg-slate-600" />
        <span
          class="h-2 w-2 shrink-0 rounded-full bg-zinc-700 shadow-[inset_0_1px_1px_rgba(255,255,255,0.25)]"
        />
      </div>
      <!-- text box -->
      <div
        class="relative z-[2] mt-[5%] flex shrink-0 basis-[22%] flex-col gap-[14%] rounded border border-slate-200 bg-slate-100 p-[6%] dark:border-white/10 dark:bg-zinc-800"
      >
        <span
          class="block h-[3.5px] w-[92%] rounded-sm bg-slate-300 opacity-50 dark:bg-slate-600"
        />
        <span
          class="block h-[3.5px] w-[78%] rounded-sm bg-slate-300 opacity-50 dark:bg-slate-600"
        />
        <span
          class="block h-[3.5px] w-[85%] rounded-sm bg-slate-300 opacity-50 dark:bg-slate-600"
        />
      </div>
    </template>

    <!-- foil holographic overlay -->
    <span v-if="foil" class="foil-fx" aria-hidden="true" />
  </div>
</template>

<style scoped>
.foil-fx {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  z-index: 4;
  background:
    linear-gradient(115deg, transparent 30%, rgba(255, 255, 255, 0.45) 44%, transparent 52%),
    linear-gradient(
      60deg,
      hsla(190, 95%, 68%, 0.55),
      hsla(265, 95%, 72%, 0.55) 22%,
      hsla(325, 95%, 70%, 0.55) 42%,
      hsla(45, 95%, 68%, 0.55) 62%,
      hsla(140, 90%, 66%, 0.55) 82%,
      hsla(190, 95%, 68%, 0.55)
    );
  background-size:
    230% 230%,
    260% 260%;
  background-position:
    calc(var(--foil, 0.5) * 100%) calc(var(--foil, 0.5) * 100%),
    calc((1 - var(--foil, 0.5)) * 100%) calc(var(--foil, 0.5) * 100%);
  mix-blend-mode: soft-light;
  opacity: 0.85;
  transition: background-position 0.12s linear;
}

.foil::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  z-index: 4;
  pointer-events: none;
  background: linear-gradient(
    115deg,
    transparent 38%,
    rgba(255, 255, 255, 0.55) 47%,
    transparent 56%
  );
  background-size: 300% 300%;
  background-position: calc(140% - var(--foil, 0.5) * 240%) 0;
  mix-blend-mode: screen;
  opacity: 0.5;
  transition: background-position 0.12s linear;
}

@media (prefers-reduced-motion: reduce) {
  .foil-fx,
  .foil::before {
    transition: none;
  }
}
</style>
