import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mockNuxtImport, mountSuspended } from '@nuxt/test-utils/runtime';
import type { Ref } from 'vue';
import SiteFooter from '~/components/SiteFooter.vue';

// `useAuth` is also called by app.vue and the register-user plugin while the Nuxt test app
// boots, so the mocked state must hold refs from the start.
const { authState } = await vi.hoisted(async () => {
  const { ref } = await import('vue');
  return {
    authState: {
      isLoaded: ref(false) as Ref<boolean>,
      isSignedIn: ref<boolean | undefined>(undefined),
    },
  };
});

mockNuxtImport('useAuth', () => () => authState);
// Signing in wakes the register-user plugin, which has no backend to call here.
mockNuxtImport('useUserService', () => () => ({ register: () => Promise.resolve() }));

const signIn = (loaded: boolean, signedIn: boolean | undefined) => {
  authState.isLoaded.value = loaded;
  authState.isSignedIn.value = signedIn;
};

const externalLink = async (text: string) => {
  const wrapper = await mountSuspended(SiteFooter);
  return wrapper.findAll('a').find((a) => a.text().includes(text))!;
};

describe('SiteFooter', () => {
  beforeEach(() => signIn(true, false));

  it('shows the « Naviguer » column to a signed-in player, with links to the right screens', async () => {
    signIn(true, true);
    const wrapper = await mountSuspended(SiteFooter);

    const nav = wrapper.get('nav[aria-label="Naviguer"]');
    expect(nav.findAll('a').map((a) => [a.text(), a.attributes('href')])).toEqual([
      ['Collection', '/collection'],
      ['Échanges', '/trade'],
      ['Rechercher', '/search'],
    ]);
  });

  it('hides the « Naviguer » column from a signed-out visitor', async () => {
    const wrapper = await mountSuspended(SiteFooter);
    expect(wrapper.find('nav[aria-label="Naviguer"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('Naviguer');
  });

  it('hides the « Naviguer » column while Clerk is still loading', async () => {
    signIn(false, undefined);
    const wrapper = await mountSuspended(SiteFooter);
    expect(wrapper.find('nav[aria-label="Naviguer"]').exists()).toBe(false);
  });

  it.each([
    ['Rejoindre le Discord', 'https://discord.gg/kd2Wwgc2w'],
    ['Fan Content Policy', 'https://company.wizards.com/fr/legal/fancontentpolicy'],
    ['Code source', 'https://github.com/michaelcoll/arcane-exchange'],
  ])('opens « %s » in a new tab', async (text, href) => {
    const link = await externalLink(text);
    expect(link.attributes('href')).toBe(href);
    expect(link.attributes('target')).toBe('_blank');
    expect(link.attributes('rel')).toBe('noopener noreferrer');
  });

  it.each([
    ['Rejoindre le Discord', 'simple-icons:discord'],
    ['Code source', 'simple-icons:github'],
  ])('marks « %s » with the brand logo', async (text, icon) => {
    const link = await externalLink(text);
    const icons = link.findAll('.iconify').flatMap((i) => i.classes());
    expect(icons).toContain(`i-${icon}`);
  });

  it('carries the Fan Content Policy notice and the data attributions', async () => {
    const wrapper = await mountSuspended(SiteFooter);
    expect(wrapper.text()).toContain(
      'Arcane Exchange est un contenu de fan non officiel autorisé par la Fan Content Policy. Non approuvé ni soutenu par Wizards. Certains éléments utilisés sont la propriété de Wizards of the Coast. ©Wizards of the Coast LLC.',
    );
    expect(wrapper.text()).toContain('Prix Cardmarket');
    expect(wrapper.text()).toContain('Données Scryfall');
    expect(wrapper.text()).toContain('Images Gatherer');
  });

  describe('copyright', () => {
    beforeEach(() => {
      vi.useFakeTimers({ toFake: ['Date'] });
    });
    afterEach(() => {
      vi.useRealTimers();
    });

    it('shows the current year', async () => {
      vi.setSystemTime(new Date('2031-03-14T12:00:00Z'));
      const wrapper = await mountSuspended(SiteFooter);
      expect(wrapper.text()).toContain('© 2031 Arcane Exchange');
    });
  });

  it('drops the navigation, the attributions and the source link under 860px', async () => {
    signIn(true, true);
    const wrapper = await mountSuspended(SiteFooter);
    const desktopOnly = wrapper.findAll('[data-desktop-only]').map((el) => el.text());
    expect(desktopOnly.join(' ')).toContain('Naviguer');
    expect(desktopOnly.join(' ')).toContain('Prix Cardmarket');
    expect(desktopOnly.join(' ')).toContain('Code source');
    for (const el of wrapper.findAll('[data-desktop-only]')) {
      expect(el.classes()).toContain('max-[860px]:hidden');
    }
  });
});
