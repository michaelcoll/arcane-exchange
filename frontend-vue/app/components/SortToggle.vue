<script setup lang="ts" generic="T extends string">
import type { SortDir } from '~/bindings/SortDir';

interface SortOption<T> {
  value: T;
  label: string;
  ascLabel: string;
  descLabel: string;
}

const props = defineProps<{
  modelValue: { sort_by: T; sort_dir: SortDir };
  options: SortOption<T>[];
}>();

const emit = defineEmits<{
  'update:modelValue': [value: { sort_by: T; sort_dir: SortDir }];
}>();

const isActive = (o: SortOption<T>) => o.value === props.modelValue.sort_by;

const onClick = (o: SortOption<T>) => {
  if (isActive(o)) {
    emit('update:modelValue', {
      sort_by: props.modelValue.sort_by,
      sort_dir: props.modelValue.sort_dir === 'asc' ? 'desc' : 'asc',
    });
  } else {
    emit('update:modelValue', { sort_by: o.value, sort_dir: props.modelValue.sort_dir });
  }
};
</script>

<template>
  <div class="flex items-center gap-2">
    <button
      v-for="o in options"
      :key="o.value"
      type="button"
      :title="isActive(o) ? `Inverser le sens (${o.label})` : `Trier par ${o.label.toLowerCase()}`"
      :aria-pressed="isActive(o)"
      :class="[
        'inline-flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium whitespace-nowrap transition-all duration-150 select-none',
        isActive(o)
          ? 'border-cyan-500/30 bg-cyan-500/10 text-cyan-700 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-300'
          : 'border-slate-200 bg-slate-100 text-slate-600 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100',
      ]"
      @click="onClick(o)"
    >
      {{ isActive(o) ? (modelValue.sort_dir === 'asc' ? o.ascLabel : o.descLabel) : o.label }}
      <Icon
        v-if="isActive(o)"
        :name="modelValue.sort_dir === 'asc' ? 'lucide:chevron-up' : 'lucide:chevron-down'"
        :size="13"
      />
    </button>
  </div>
</template>
