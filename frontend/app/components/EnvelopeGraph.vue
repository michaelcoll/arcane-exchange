<script setup lang="ts">
type EnvelopePoint = { low: number; avg: number; trend: number; day: number; label: string };

const props = withDefaults(
  defineProps<{
    data: EnvelopePoint[];
    detail?: boolean;
  }>(),
  { detail: false },
);

const rootRef = ref<HTMLElement | null>(null);
const dim = ref({ w: 0, h: 0 });
const hover = ref<number | null>(null);
let ro: ResizeObserver | null = null;

onMounted(() => {
  const node = rootRef.value;
  if (!node || typeof ResizeObserver === 'undefined') return;
  ro = new ResizeObserver((entries) => {
    const r = entries[0]?.contentRect;
    if (!r) return;
    dim.value = { w: Math.round(r.width), h: Math.round(r.height) };
  });
  ro.observe(node);
});

onUnmounted(() => ro?.disconnect());

// Cubic Hermite -> bezier smoothing. Slopes come from the neighbours' real x distance, so
// unevenly spaced points (gaps in the history) never make the curve fold back on itself;
// with even spacing this is plain Catmull-Rom.
const envSmooth = (pts: [number, number][]) => {
  if (pts.length < 2) return '';
  const slope = (a: [number, number], b: [number, number]) =>
    b[0] === a[0] ? 0 : (b[1] - a[1]) / (b[0] - a[0]);
  let d = `M${pts[0]![0].toFixed(1)},${pts[0]![1].toFixed(1)}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i - 1] ?? pts[i]!;
    const p1 = pts[i]!;
    const p2 = pts[i + 1]!;
    const p3 = pts[i + 2] ?? p2;
    const third = (p2[0] - p1[0]) / 3;
    const c1x = p1[0] + third;
    const c1y = p1[1] + slope(p0, p2) * third;
    const c2x = p2[0] - third;
    const c2y = p2[1] - slope(p1, p3) * third;
    d += `C${c1x.toFixed(1)},${c1y.toFixed(1)} ${c2x.toFixed(1)},${c2y.toFixed(1)} ${p2[0].toFixed(1)},${p2[1].toFixed(1)}`;
  }
  return d;
};

const n = computed(() => props.data.length);
const minVal = computed(() => Math.min(...props.data.map((d) => Math.min(d.low, d.avg, d.trend))));
const maxVal = computed(() => Math.max(...props.data.map((d) => Math.max(d.low, d.avg, d.trend))));
const pad = computed(() => (maxVal.value - minVal.value) * 0.12 || 1);
const lo0 = computed(() => minVal.value - pad.value);
const hi0 = computed(() => maxVal.value + pad.value);

const padT = 10;
const padR = computed(() => (props.detail ? 12 : 4));
const padL = computed(() => (props.detail ? 46 : 4));
const iw = computed(() => Math.max(1, dim.value.w - padL.value - padR.value));

// The x axis is in days, not in entries: a gap in the history keeps its real length.
const days = computed(() => props.data.map((d) => d.day));
const firstDay = computed(() => days.value[0] ?? 0);
const lastDay = computed(() => days.value[n.value - 1] ?? 0);

const tickN = 6;
const ticks = computed(() => dayTicks(firstDay.value, lastDay.value, tickN));
// A date label ("27 sept.") is about 50 px wide: below this spacing, neighbours would touch,
// so the labels are tilted and the plot leaves them more room underneath.
const MIN_FLAT_TICK_SPACING = 60;
const tiltTicks = computed(
  () => ticks.value.length > 1 && iw.value / (ticks.value.length - 1) < MIN_FLAT_TICK_SPACING,
);

const padB = computed(() => (props.detail ? (tiltTicks.value ? 44 : 22) : 6));
const ih = computed(() => Math.max(1, dim.value.h - padT - padB.value));

const xDay = (day: number) =>
  padL.value + ((day - firstDay.value) / (lastDay.value - firstDay.value || 1)) * iw.value;
const x = (i: number) => xDay(days.value[i] ?? firstDay.value);
const y = (v: number) => padT + ih.value - ((v - lo0.value) / (hi0.value - lo0.value)) * ih.value;

const ready = computed(() => dim.value.w > 0 && dim.value.h > 0);

const topD = computed(() => {
  if (!ready.value) return '';
  return envSmooth(props.data.map((d, i) => [x(i), y(d.avg)]));
});
const botD = computed(() => {
  if (!ready.value) return '';
  return envSmooth(props.data.map((d, i) => [x(i), y(d.low)]));
});
const midD = computed(() => {
  if (!ready.value) return '';
  return envSmooth(props.data.map((d, i) => [x(i), y(d.trend)]));
});
const areaD = computed(() => {
  if (!ready.value) return '';
  const botPts = props.data.map((d, i) => [x(i), y(d.low)] as [number, number]);
  const botRev = envSmooth([...botPts].reverse());
  const last = botPts[botPts.length - 1]!;
  return `${topD.value} L${last[0].toFixed(1)},${last[1].toFixed(1)} ${botRev.slice(botRev.indexOf('C'))}`;
});

const onMove = (e: MouseEvent) => {
  if (!props.detail) return;
  const r = rootRef.value?.getBoundingClientRect();
  if (!r) return;
  const px = e.clientX - r.left;
  const day = firstDay.value + ((px - padL.value) / iw.value) * (lastDay.value - firstDay.value);
  hover.value = nearestIndex(days.value, day);
};

const hoverPoint = computed(() => (hover.value != null ? props.data[hover.value] : null));
const tooltipStyle = computed(() => {
  if (hover.value == null || !hoverPoint.value) return {};
  return {
    left: Math.max(64, Math.min(dim.value.w - 70, x(hover.value))) + 'px',
    top: y(hoverPoint.value.avg) + 'px',
  };
});
</script>

<template>
  <div ref="rootRef" class="relative h-full w-full" @mousemove="onMove" @mouseleave="hover = null">
    <svg
      v-if="ready"
      :width="dim.w"
      :height="dim.h"
      :viewBox="`0 0 ${dim.w} ${dim.h}`"
      class="block"
    >
      <line
        v-for="(f, k) in [0.25, 0.5, 0.75]"
        :key="k"
        class="stroke-slate-900/5 dark:stroke-white/5"
        stroke-width="1"
        :class="detail ? 'opacity-100' : 'opacity-0'"
        :style="{ transition: 'opacity .35s .1s' }"
        :x1="padL"
        :y1="padT + ih * f"
        :x2="dim.w - padR"
        :y2="padT + ih * f"
      />
      <path class="fill-primary/30 stroke-none" :d="areaD" />
      <path class="stroke-primary/40 fill-none [stroke-width:1.4]" :d="topD" />
      <path class="stroke-primary/40 fill-none [stroke-width:1.4]" :d="botD" />
      <path
        class="stroke-primary fill-none [stroke-width:2.6] drop-shadow-[0_0_5px_var(--primary-glow)]"
        :d="midD"
      />
      <line
        v-if="detail && hover != null"
        class="stroke-slate-400 stroke-1 [stroke-dasharray:3_3] dark:stroke-white/25"
        :x1="x(hover)"
        :y1="padT"
        :x2="x(hover)"
        :y2="padT + ih"
      />
    </svg>

    <template v-if="ready">
      <span
        class="text-2xs absolute top-1 left-1 font-mono text-slate-400 transition-opacity duration-200 dark:text-slate-500"
        :class="detail ? 'opacity-100 delay-100' : 'opacity-0'"
        >{{ formatChartPrice(hi0) }}</span
      >
      <span
        class="text-2xs absolute left-1 font-mono text-slate-400 transition-opacity duration-200 dark:text-slate-500"
        :class="detail ? 'opacity-100 delay-100' : 'opacity-0'"
        :style="{ bottom: padB + 2 + 'px' }"
        >{{ formatChartPrice(lo0) }}</span
      >
      <!-- tilted labels pivot on their end, so each date still ends right under its tick -->
      <span
        v-for="day in ticks"
        :key="day"
        class="text-2xs absolute font-mono whitespace-nowrap text-slate-400 transition-opacity duration-200 dark:text-slate-500"
        :class="[
          detail ? 'opacity-100 delay-100' : 'opacity-0',
          tiltTicks ? 'origin-top-right' : 'bottom-1',
        ]"
        :style="
          tiltTicks
            ? {
                left: xDay(day) + 'px',
                top: padT + ih + 6 + 'px',
                transform: 'translateX(-100%) rotate(-40deg)',
              }
            : { left: xDay(day) + 'px', transform: 'translateX(-50%)' }
        "
        >{{ formatDay(day) }}</span
      >
    </template>

    <div
      v-if="detail && hover != null && hoverPoint"
      class="pointer-events-none absolute z-[6] min-w-[128px] -translate-x-1/2 -translate-y-[112%] rounded-[10px] border border-slate-300 bg-white px-2.5 py-2 shadow-lg dark:border-white/15 dark:bg-zinc-800"
      :style="tooltipStyle"
    >
      <div
        class="text-2xs mb-1 font-mono tracking-wide text-slate-400 uppercase dark:text-slate-500"
      >
        {{ hoverPoint.label }}
      </div>
      <div class="flex items-center justify-between gap-3.5 text-xs leading-relaxed">
        <span class="flex items-center gap-1.5 text-slate-500 dark:text-slate-400"
          ><i class="bg-primary-soft inline-block h-2 w-2 rounded-sm" />avg</span
        >
        <b class="font-mono font-semibold">{{ formatChartPrice(hoverPoint.avg) }}</b>
      </div>
      <div class="flex items-center justify-between gap-3.5 text-xs leading-relaxed">
        <span class="flex items-center gap-1.5 text-slate-500 dark:text-slate-400"
          ><i class="bg-primary inline-block h-2 w-2 rounded-sm" />trend</span
        >
        <b class="font-mono font-semibold">{{ formatChartPrice(hoverPoint.trend) }}</b>
      </div>
      <div class="flex items-center justify-between gap-3.5 text-xs leading-relaxed">
        <span class="flex items-center gap-1.5 text-slate-500 dark:text-slate-400"
          ><i class="bg-primary-dim inline-block h-2 w-2 rounded-sm" />low</span
        >
        <b class="font-mono font-semibold">{{ formatChartPrice(hoverPoint.low) }}</b>
      </div>
    </div>
  </div>
</template>
