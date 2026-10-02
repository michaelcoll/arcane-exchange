<script setup lang="ts">
/* Footer du site, d'après le `SiteFooter` de la maquette. Sous 860px, la navigation, les
 * attributions et le lien vers le code source disparaissent (`data-desktop-only`) : il ne reste que
 * la marque, la mention Fan Content Policy, la carte Discord et le copyright. */
const DISCORD_URL = 'https://discord.gg/kd2Wwgc2w';
const FCP_URL = 'https://company.wizards.com/fr/legal/fancontentpolicy';
const SOURCE_URL = 'https://github.com/michaelcoll/arcane-exchange';

const { isSignedIn, isLoaded } = useAuth();
const { getStats } = useStatsService();

const year = new Date().getFullYear();

// The footer is not critical: if `/stats` fails, the stats and the pill simply stay hidden — no
// toast, no error state.
const { data: stats } = useAsyncData('stats', getStats, { lazy: true });

const freshness = computed(() =>
  stats.value?.last_price_date ? priceFreshness(stats.value.last_price_date) : null,
);

const formatCount = (value: number) => new Intl.NumberFormat('fr-FR').format(value);

const statItems = computed(() =>
  stats.value
    ? [
        { label: 'Cartes référencées', value: formatCount(stats.value.card_number) },
        { label: 'Prix enregistrés', value: formatCount(stats.value.card_price_number) },
        { label: 'Taille de la base', value: `${formatCount(stats.value.db_size_mb)} Mo` },
      ]
    : [],
);

// Same tones as `Trade/StatusPill`, in its `sm` size.
const pillTone: Record<PriceFreshness['tone'], string> = {
  good: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/30 dark:bg-emerald-400/10 dark:text-emerald-300',
  muted:
    'border-slate-300 bg-slate-100 text-slate-500 dark:border-white/15 dark:bg-white/5 dark:text-slate-400',
};

const label =
  'font-mono text-[10.5px] font-medium tracking-[0.13em] whitespace-nowrap text-[var(--ink-3)] uppercase';
const link = 'text-[13px] text-[var(--ink-2)] transition-colors duration-150 hover:text-primary';
const dot = 'h-[3px] w-[3px] flex-none rounded-full bg-[var(--ink-4)]';
</script>

<template>
  <footer
    class="relative z-[1] mt-[26px] border-t border-[var(--line)] bg-[color-mix(in_srgb,var(--surface)_42%,transparent)] backdrop-blur-[10px] before:absolute before:inset-x-0 before:-top-px before:h-px before:bg-[linear-gradient(90deg,transparent,var(--primary-line)_22%,var(--secondary-line)_70%,transparent)] before:content-['']"
  >
    <div class="mx-auto max-w-[1180px] px-[26px] max-[860px]:px-4">
      <div
        class="flex items-start gap-[60px] pt-[30px] pb-[26px] max-[860px]:flex-col max-[860px]:gap-7 max-[860px]:pt-[26px] max-[860px]:pb-[22px]"
      >
        <div class="flex min-w-0 flex-1 flex-col gap-4">
          <div class="flex flex-wrap items-center gap-3">
            <span
              class="border-primary-line grid h-[26px] w-[26px] flex-none place-items-center rounded-lg border bg-[linear-gradient(150deg,color-mix(in_oklch,var(--primary)_26%,var(--surface)),var(--surface-2))]"
            >
              <AppLogo class="h-4 w-4" stroke-width="2.4" />
            </span>
            <span class="font-display text-[14.5px] font-semibold tracking-[-0.01em]"
              >Arcane <b class="text-primary font-semibold">Exchange</b></span
            >
            <span
              v-if="freshness"
              data-price-freshness
              :data-tone="freshness.tone"
              :class="[
                'inline-flex items-center gap-1.5 rounded-full border px-2.5 py-[3px] font-mono text-[10px] font-semibold tracking-wider whitespace-nowrap',
                pillTone[freshness.tone],
              ]"
            >
              <span
                :class="[
                  'h-1.5 w-1.5 flex-none rounded-full bg-current',
                  freshness.tone === 'good' && 'shadow-[0_0_8px_currentColor]',
                ]"
              />{{ freshness.label }}</span
            >
          </div>

          <div
            v-if="statItems.length"
            class="flex items-stretch max-[860px]:flex-wrap max-[860px]:gap-y-3.5"
          >
            <div
              v-for="stat in statItems"
              :key="stat.label"
              data-stat
              class="flex flex-col gap-1 border-l border-[var(--line)] px-5 first:border-l-0 first:pl-0 max-[560px]:px-3.5 max-[560px]:first:pl-0"
            >
              <span :class="label">{{ stat.label }}</span>
              <b class="font-display text-[19px] font-semibold tracking-[-0.02em]">{{
                stat.value
              }}</b>
            </div>
          </div>

          <p
            class="m-0 max-w-[520px] text-justify text-[11px] leading-[1.55] hyphens-auto text-[var(--ink-3)]"
          >
            Arcane Exchange est un contenu de fan non officiel autorisé par la
            <a
              :href="FCP_URL"
              target="_blank"
              rel="noopener noreferrer"
              class="hover:text-primary text-[var(--ink-2)] underline underline-offset-2 transition-colors duration-150"
              >Fan Content Policy</a
            >. Non approuvé ni soutenu par Wizards. Certains éléments utilisés sont la propriété de
            Wizards of the Coast. ©Wizards of the Coast LLC.
          </p>
        </div>

        <div
          class="flex flex-none items-start gap-14 max-[860px]:flex-wrap max-[860px]:justify-center max-[860px]:self-stretch"
        >
          <a
            :href="DISCORD_URL"
            target="_blank"
            rel="noopener noreferrer"
            class="border-secondary-line hover:border-primary-line flex items-center gap-3.5 rounded-[var(--r-md)] border bg-[color-mix(in_srgb,var(--surface-2)_70%,transparent)] px-4 py-3.5 text-[var(--ink)] transition-colors duration-150 max-[860px]:w-full"
          >
            <span
              class="text-secondary grid h-[38px] w-[38px] flex-none place-items-center rounded-[10px] bg-[color-mix(in_oklch,var(--secondary)_16%,var(--surface))]"
            >
              <Icon name="simple-icons:discord" :size="20" />
            </span>
            <span class="flex flex-col gap-0.5 max-[860px]:flex-1">
              <b class="font-display text-sm font-semibold">Rejoindre le Discord</b>
              <span class="text-xs text-[var(--ink-3)]">Entraide, échanges et nouveautés</span>
            </span>
            <Icon name="lucide:arrow-up-right" :size="16" class="text-[var(--ink-3)]" />
          </a>

          <nav
            v-if="isLoaded && isSignedIn"
            aria-label="Naviguer"
            data-desktop-only
            class="flex flex-col gap-2.5 max-[860px]:hidden"
          >
            <div :class="[label, 'mb-0.5']">Naviguer</div>
            <NuxtLink to="/collection" :class="link">Collection</NuxtLink>
            <NuxtLink to="/trade" :class="link">Échanges</NuxtLink>
            <NuxtLink to="/search" :class="link">Rechercher</NuxtLink>
          </nav>
        </div>
      </div>

      <div
        class="flex flex-wrap items-center justify-between gap-[18px] border-t border-[var(--line-3)] py-[13px]"
      >
        <span class="font-mono text-[11.5px] tracking-[0.01em] text-[var(--ink-3)]"
          >© {{ year }} Arcane Exchange</span
        >
        <div
          data-desktop-only
          class="flex flex-wrap items-center gap-2.5 font-mono text-[11.5px] text-[var(--ink-3)] max-[860px]:hidden"
        >
          <span>Prix Cardmarket</span>
          <i :class="dot" />
          <span>Données Scryfall</span>
          <i :class="dot" />
          <span>Images Gatherer</span>
          <i :class="dot" />
          <a
            :href="SOURCE_URL"
            target="_blank"
            rel="noopener noreferrer"
            class="hover:text-primary inline-flex items-center gap-1.5 transition-colors duration-150"
          >
            <Icon name="simple-icons:github" :size="12" />Code source
          </a>
        </div>
      </div>
    </div>
  </footer>
</template>
