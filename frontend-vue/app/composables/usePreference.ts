import type { Preference } from '~/utils/display-prefs';

/** A ref seeded from localStorage and written back on each change (see `loadPreference`). */
export const usePreference = <T extends string>(pref: Preference<T>): Ref<T> => {
  const value = ref(loadPreference(pref)) as Ref<T>;
  watch(value, (v) => savePreference(pref, v));
  return value;
};
