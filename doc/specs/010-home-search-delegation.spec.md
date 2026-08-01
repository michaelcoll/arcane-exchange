# Spec : Délégation de la recherche depuis la page d'accueil

## Contexte

La page d'accueil (`/`) propose un champ de recherche de carte (mode nom et mode decklist).
Actuellement, cliquer "Chercher" navigue vers `/find` **sans transmettre** la requête ni le mode :
l'utilisateur arrive sur une page vide et doit retaper sa recherche.
l'utilisateur arrive sur une page vide et doit retaper sa recherche.

## Objectif

Quand l'utilisateur lance une recherche depuis la home, il doit atterrir sur `/search` avec la
recherche **déjà exécutée** (mode + requête), sans action supplémentaire.

Pour le mode decklist, le contenu du textarea de la home est transporté et pré-remplit le
formulaire de `/search`.

## Solution

### Transport des données

- **Mode nom** : requête dans les paramètres de URL (`?q=...&mode=name`).
- **Mode decklist** : contenu du textarea dans `sessionStorage` (clé dédiée, ex: `tae_decklist_pending`),
  transporté via la navigation.

### Navigation depuis la home

1. La home détecte le mode courant (`name` ou `decklist`).
2. **Mode nom** : `navigateTo('/search', { query: { q: q.value, mode: mode.value } })`.
3. **Mode decklist** : stockage du contenu dans `sessionStorage`, puis `navigateTo('/search')`.
4. **Mode decklist avec contenu vide** : navigation simple sans stockage.

### Réception sur `/search`

Au montage de la page `/search` :

1. Vérifier si des paramètres `q` et `mode` sont présents dans l'URL.
2. Si oui : définir `q` et `mode` en conséquence, **exécuter la recherche automatiquement**,
   puis effacer les params de l'URL (pour ne pas polluer l'historique).
3. Vérifier `sessionStorage` pour un decklist en attente. Si trouvé :
   définir `mode = 'decklist'`, pré-remplir le textarea, **effacer** le stockage.
4. Dans tous les autres cas : comportement actuel (formulaire vide, chargement initial).

### Comportement combiné

Si l'utilisateur navigue depuis la home en mode nom **et** qu'il y a un decklist résidu
dans le sessionStorage (ex: navigation rapide entre les modes), le paramètre `q` de l'URL
prime sur le sessionStorage.

### Modifications de route

Renommer la page de recherche de `/find` à `/search` :

- Renommer `frontend-vue/app/pages/find/index.vue` → `frontend-vue/app/pages/search/index.vue`.
- Mettre à jour **tous les liens** pointant vers `/find` (navigation principale, bottom nav, liens internes, etc.).
- La nouvelle route est `/search`.

Seul le traitement des query params est ajouté sur la page (feature addition, pas breaking).

## Cas d'erreurs

- **Paramètre `q` vide** (`/search?q=`) : Ignoré, pas de recherche automatique.
- **Paramètre `mode` invalide** (`/search?mode=foo`) : ignoré, mode par défaut conservé.
- **sessionStorage corrompu** : lecture protégée par try/catch, valeur ignorée en cas d'erreur.
- **Longueur du decklist trop grande pour sessionStorage** : pas de limite technique dans le navigateur
  (~5-10 Mo), mais on peut ajouter un log en cas de contenu très volumineux (>50 ko).

## Critères d'acceptance

- [ ] Depuis la home en mode "Nom", taper "Vampiric Tutor" et cliquer "Chercher" → l'utilisateur arrive sur `/search?q=Vampiric+Tutor&mode=name` avec les résultats de "Vampiric Tutor" affichés immédiatement.
- [ ] L'URL se nettoie après le chargement : `?q=...&mode=...` est retiré, l'URL devient `/search`.
- [ ] Le refresh de la page `/search` (après nettoyage) recharge la page avec un formulaire vide (comportement par défaut), pas la recherche précédente.
- [ ] Depuis la home en mode "Decklist" avec contenu collé, cliquer "Trouver les joueurs" → arrivée sur `/find` en mode decklist avec le textarea pré-rempli. Le contenu a été supprimé du sessionStorage.
- [ ] Depuis la home en mode "Decklist" avec textarea vide, cliquer "Trouver les joueurs" → arrivée sur `/find` en mode decklist avec textarea vide.
- [ ] Naviguer manuellement à `/search?q=Black+Market+Connections&mode=name` → la recherche s'exécute automatiquement.
- [ ] Naviguer à `/search?q=&mode=name` (query vide) → la recherche automatique n'est pas déclenchée.
- [ ] Naviguer à `/search?mode=invalid` → le mode est ignoré, valeur par défaut conservée.
- [ ] Naviguer de home→find en mode nom, puis immédiatement back→home, puis home→find en mode decklist avec contenu → le decklist est bien pré-rempli (le `q` précédent n'interfère pas).
