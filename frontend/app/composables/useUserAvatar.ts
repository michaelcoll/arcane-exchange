import type { MaybeRefOrGetter } from 'vue';

// Cache session partagé des avatars, indexé par username : une seule requête
// GET /user/{username} par joueur, même dans les listes (offres, recherche,
// échanges). Le chargement n'est déclenché qu'au montage côté client
// (onMounted), jamais pendant le SSR — le cache module-level reste donc vide
// côté serveur et ne peut pas fuiter entre deux sessions utilisateur.
const avatars = new Map<string, string | null>();
const inFlight = new Map<string, Promise<string | null>>();

export const useUserAvatar = (username: MaybeRefOrGetter<string>) => {
  const target = computed(() => toValue(username));
  const avatar = ref<string | null>(null);
  const { getUserProfile } = useUserService();

  const load = async () => {
    const u = target.value;
    if (avatars.has(u)) {
      avatar.value = avatars.get(u) ?? null;
      return;
    }

    let promise = inFlight.get(u);
    if (!promise) {
      promise = getUserProfile(u)
        .then((profile) => profile.avatar_url)
        .catch((error: { statusCode?: number; response?: { status?: number } }) => {
          // Seul un vrai 404 (profil sans avatar / introuvable) est mis en cache :
          // une erreur transitoire (réseau, 5xx, token expiré) ne doit pas figer
          // l'utilisateur sur les initiales pour le reste de la session.
          if (error?.statusCode === 404 || error?.response?.status === 404) {
            return null;
          }
          throw error;
        });
      inFlight.set(u, promise);
    }

    let result: string | null;
    try {
      result = await promise;
    } catch {
      inFlight.delete(u);
      return;
    }
    inFlight.delete(u);
    avatars.set(u, result);
    // La cible a changé pendant la requête : résultat obsolète, on ne l'affiche pas.
    if (target.value !== u) return;
    avatar.value = result;
  };

  // Changement de joueur : on revient aux initiales puis on charge le nouvel avatar.
  // Sans passage par les initiales, l'avatar de l'ancien joueur resterait affiché
  // pendant la requête, et définitivement si elle échoue.
  watch(target, () => {
    avatar.value = null;
    load();
  });

  return { avatar, load };
};
