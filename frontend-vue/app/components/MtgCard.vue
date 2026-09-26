<script setup lang="ts">
const props = defineProps<{
  /** The card's stored image (`image_url`), `null` while it is pending. */
  imageUrl?: string | null;
  /** The back of a double-faced card (`image_back_url`). */
  imageBackUrl?: string | null;
  /** Offers to flip a double-faced card to its back. */
  flippable?: boolean;
  qty?: number;
  clickable?: boolean;
  mini?: boolean;
  foil?: boolean;
  name?: string;
  size?: 'sm' | 'md' | 'lg';
  reserved?: boolean;
}>();

const emit = defineEmits(['click']);

/** Generic card back, shown for a card without an image or whose image fails to load. */
const CARD_BACK = '/card-back.webp';

const frontFailed = ref(false);
const backFailed = ref(false);
const flipped = ref(false);

watch(
  () => [props.imageUrl, props.imageBackUrl],
  () => {
    frontFailed.value = false;
    backFailed.value = false;
    flipped.value = false;
  },
);

const frontSrc = computed(() => (!frontFailed.value && props.imageUrl) || CARD_BACK);
const backSrc = computed(() => (!backFailed.value && props.imageBackUrl) || CARD_BACK);
const canFlip = computed(() => !!props.flippable && !!props.imageBackUrl);

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
      'relative aspect-[5/7] [perspective:1200px]',
      'ease transition-[transform,box-shadow] duration-200',
      'rounded-[4%]',
      clickable ? 'cursor-pointer hover:-translate-y-1 hover:shadow-xl' : '',
      foil ? 'foil' : '',
      reserved ? 'ring-2 ring-violet-500 dark:ring-violet-400' : '',
    ]"
    :title="name"
    @click="emit('click')"
  >
    <span
      v-if="qty != null"
      class="absolute top-1.5 right-1.5 z-[5] rounded-full border border-white/20 bg-black/60 px-1.5 py-0.5 font-mono text-xs font-semibold text-zinc-100 backdrop-blur-sm"
      >×{{ qty }}</span
    >

    <span
      v-if="reserved && !mini"
      class="text-2xs absolute top-1.5 left-1.5 z-[5] inline-flex items-center gap-1 rounded-full border border-violet-400/40 bg-violet-500/20 px-1.5 py-0.5 font-mono font-bold text-violet-100 backdrop-blur-sm"
    >
      <Icon name="lucide:lock" :size="10" />
      Réservée
    </span>

    <!-- faces: the back is only mounted for a card that can be flipped to it -->
    <div
      :class="[
        'relative h-full w-full transition-transform duration-500 [transform-style:preserve-3d] motion-reduce:transition-none',
        flipped ? '[transform:rotateY(180deg)]' : '',
      ]"
    >
      <img
        class="absolute inset-0 block h-full w-full rounded-[4%] object-cover select-none [backface-visibility:hidden]"
        :src="frontSrc"
        :alt="name ?? ''"
        loading="lazy"
        draggable="false"
        @error="frontFailed = true"
      />
      <img
        v-if="canFlip"
        class="absolute inset-0 block h-full w-full [transform:rotateY(180deg)] rounded-[4%] object-cover select-none [backface-visibility:hidden]"
        :src="backSrc"
        :alt="name ? `${name} (verso)` : ''"
        loading="lazy"
        draggable="false"
        @error="backFailed = true"
      />
    </div>

    <button
      v-if="canFlip"
      type="button"
      data-testid="flip"
      :aria-pressed="flipped"
      :aria-label="flipped ? 'Voir le recto' : 'Voir le verso'"
      :title="flipped ? 'Voir le recto' : 'Voir le verso'"
      class="absolute right-2.5 bottom-2.5 z-[5] grid h-9 w-9 place-items-center rounded-full bg-black/60 text-zinc-100 backdrop-blur-sm transition-colors duration-150 hover:bg-cyan-500/80 focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:outline-none"
      @click.stop="flipped = !flipped"
    >
      <Icon name="lucide:refresh-cw" :size="16" />
    </button>

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
