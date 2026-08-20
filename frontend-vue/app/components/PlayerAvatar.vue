<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    username: string;
    size?: 'sm' | 'lg';
  }>(),
  { size: 'sm' },
);

// Getter prop : le composable suit les changements de joueur (watch interne)
// et repasse par les initiales à chaque nouveau username.
const { avatar, load } = useUserAvatar(() => props.username);
const initials = computed(() => props.username.slice(0, 2).toUpperCase());
const imageFailed = ref(false);
const showImage = computed(() => Boolean(avatar.value) && !imageFailed.value);

// Changement de joueur : on retente l'image, sinon l'échec du joueur
// précédent restait figé sur les initiales pour tous les suivants.
watch(
  () => props.username,
  () => {
    imageFailed.value = false;
  },
);

onMounted(() => {
  load();
});
</script>

<template>
  <img
    v-if="showImage"
    :src="avatar ?? undefined"
    :alt="`Avatar de ${username}`"
    :class="[
      'shrink-0 rounded-full border border-slate-300 object-cover dark:border-white/15',
      size === 'lg' ? 'h-14 w-14' : 'h-8 w-8',
    ]"
    @error="imageFailed = true"
  />
  <div
    v-else
    :class="[
      'grid shrink-0 place-items-center rounded-full border border-slate-300 bg-slate-100 font-mono text-slate-600 dark:border-white/15 dark:bg-zinc-800 dark:text-slate-300',
      size === 'lg' ? 'h-14 w-14 text-base' : 'h-8 w-8 text-xs',
    ]"
  >
    {{ initials }}
  </div>
</template>
