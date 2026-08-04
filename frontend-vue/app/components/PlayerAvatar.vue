<script setup lang="ts">
withDefaults(
  defineProps<{
    initials: string;
    size?: 'sm' | 'lg';
  }>(),
  { size: 'sm' },
);
</script>

<template>
  <div
    :class="[
      'grid shrink-0 place-items-center rounded-full border border-slate-300 bg-slate-100 font-mono text-slate-600 dark:border-white/15 dark:bg-zinc-800 dark:text-slate-300',
      size === 'lg' ? 'h-14 w-14 text-base' : 'h-8 w-8 text-xs',
    ]"
  >
    {{ initials }}
  </div>
</template>
