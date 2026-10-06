export type ThemePreference = 'auto' | 'light' | 'dark';

export const useAppTheme = () => {
  const themePreference = useState<ThemePreference>('tae_theme_pref', () => 'auto');
  const prefersLight = useMediaQuery('(prefers-color-scheme: light)');

  const theme = computed<'dark' | 'light'>(() =>
    themePreference.value === 'auto'
      ? prefersLight.value
        ? 'light'
        : 'dark'
      : themePreference.value,
  );

  const cycleThemePreference = () => {
    themePreference.value =
      themePreference.value === 'auto'
        ? 'light'
        : themePreference.value === 'light'
          ? 'dark'
          : 'auto';
  };

  // Colours come from the Clerk dashboard; light/dark follows `color-scheme` (main.css).
  const clerkAppearance = {
    variables: { fontFamily: "'Hanken Grotesk', system-ui, sans-serif" },
  };

  return { theme, themePreference, cycleThemePreference, clerkAppearance };
};
