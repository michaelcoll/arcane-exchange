<script setup lang="ts">
interface Option {
  value: string;
  label?: string;
  icon?: string;
  tone?: string;
  title?: string;
  kbd?: string; // touche brute (1, 2 … K), le modificateur s'ajoute selon l'OS
}

const props = withDefaults(
  defineProps<{
    modelValue: string;
    options: Option[];
    size?: 'sm';
    shortcuts?: boolean; // active les raccourcis clavier (déf. false)
  }>(),
  { shortcuts: false },
);

const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const containerRef = ref<HTMLElement | null>(null);
const thumb = ref({ left: 3, width: 0, opacity: 0 });
const currentIndex = computed(() => props.options.findIndex((o) => o.value === props.modelValue));

/* ---------- OS detection (⌥ sur Mac, Alt ailleurs) ---------- */
const isMac = computed(() => {
  const ua = navigator as Navigator & { userAgentData?: { platform: string } };
  if (ua.userAgentData?.platform.startsWith('Mac')) return true;
  if (/Macintosh|MacOSX|Mac OS X/.test(navigator.userAgent)) return true;
  return false;
});

const modifierLabel = computed(() => (isMac.value ? '⌥' : 'Alt'));

const updateThumb = () => {
  const el = containerRef.value;
  if (!el) return;
  const idx = currentIndex.value < 0 ? 0 : currentIndex.value;
  const btn = el.querySelectorAll('button')[idx] as HTMLElement | undefined;
  if (btn) {
    thumb.value = { left: btn.offsetLeft, width: btn.offsetWidth, opacity: 1 };
  }
};

let ro: ResizeObserver | null = null;

onMounted(() => {
  nextTick(updateThumb);
  ro = new ResizeObserver(() => nextTick(updateThumb));
  if (containerRef.value) ro.observe(containerRef.value);
});

onUnmounted(() => ro?.disconnect());

watch(
  () => [props.modelValue, props.options.length],
  () => nextTick(updateThumb),
);

const btnActiveColor = (o: Option) => {
  if (o.value !== props.modelValue) return 'text-slate-600 dark:text-slate-300';
  if (o.tone === 'cyan') return 'text-cyan-600 dark:text-cyan-400';
  if (o.tone === 'vio') return 'text-violet-500 dark:text-violet-300';
  return 'text-slate-800 dark:text-slate-100';
};

const kbdClasses = (o: Option) =>
  o.value === props.modelValue
    ? // actif : light bordure/texte plus clairs (bg inchangé) ; dark idem sur fond noir 22%
      'border-[var(--line-2)] bg-[var(--surface)] text-[var(--ink)] dark:border-[color-mix(in_oklch,white_9%,transparent)] dark:bg-[color-mix(in_srgb,black_22%,transparent)] dark:text-[oklch(0.55_0.006_250)]'
    : // inactif : light sur fond blanc, bordure line, texte ink-2 ; dark bg noir 22%, bordure line-2, texte ink-4
      'border-[var(--line)] bg-[var(--surface)] text-[var(--ink-2)] dark:border-[color-mix(in_oklch,white_14%,transparent)] dark:bg-[color-mix(in_srgb,black_22%,transparent)] dark:text-[oklch(0.42_0.006_250)]';

/* ---------- Keyboard shortcuts (port de useModeShortcut) ----------
 * Alt+1..9 → sélectionne l'option n°N  (⌥ sur Mac)
 * Alt+K    → cycle vers l'option suivante
 * alt seul comme modérateur (Cmd/Ctrl+chiffre = onglets navigateur)
 * on utilise e.code car Alt+chiffre sur Mac produit des caractères accentués
 * les raccourcis sont désactivables via le prop `shortcuts`
 */
const handleKeydown = (e: KeyboardEvent) => {
  if (!e.altKey || e.metaKey || e.ctrlKey) return;

  if (e.code === 'KeyK') {
    e.preventDefault();
    const idx = (currentIndex.value + 1) % props.options.length;
    emit('update:modelValue', props.options[idx]!.value);
    return;
  }

  const m = /^(?:Digit|Numpad)([1-9])$/.exec(e.code ?? '');
  const n = m ? Number(m[1]) : NaN;
  if (n >= 1 && n <= props.options.length) {
    e.preventDefault();
    emit('update:modelValue', props.options[n - 1]!.value);
  }
};

onMounted(() => {
  if (!props.shortcuts) return;
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div
    ref="containerRef"
    class="relative inline-flex gap-0.5 rounded-xl border border-slate-300 bg-slate-200 p-1 dark:border-white/10 dark:bg-black/20"
  >
    <span
      class="absolute top-0.5 bottom-0.5 z-0 rounded-lg border border-slate-300 bg-white shadow-md [transition:left_0.26s_cubic-bezier(0.5,1.3,0.5,1),width_0.26s_cubic-bezier(0.5,1.3,0.5,1)] dark:border-white/15 dark:bg-zinc-800"
      :style="{ left: thumb.left + 'px', width: thumb.width + 'px', opacity: thumb.opacity }"
    />
    <button
      v-for="o in options"
      :key="o.value"
      :title="o.title"
      :aria-label="o.title"
      :class="[
        'relative z-10 inline-flex items-center justify-center rounded-lg font-semibold whitespace-nowrap transition-colors duration-200',
        size === 'sm' ? 'px-2.5 py-1.5 text-xs' : 'px-3.5 py-2 text-xs',
        btnActiveColor(o),
      ]"
      @click="emit('update:modelValue', o.value)"
    >
      <Icon v-if="o.icon" :name="o.icon" size="15" />
      <template v-else>{{ o.label }}</template>
      <span
        v-if="o.kbd"
        :class="[
          'ml-[7px] hidden rounded-[4px] border px-[5px] py-[2.5px] font-mono text-[10.5px] leading-none transition-colors duration-150 md:inline-block',
          kbdClasses(o),
        ]"
        >{{ modifierLabel }}{{ o.kbd }}</span
      >
    </button>
  </div>
</template>
