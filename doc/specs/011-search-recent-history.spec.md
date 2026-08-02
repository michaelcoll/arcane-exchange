# Spec : Recherches récentes dynamiques depuis localStorage

## Contexte

La page d'accueil (`/`) affiche une section "récents" sous le champ de recherche en mode nom.
Actuellement, cette liste est en dur (`['Vampiric Tutor', 'Black Market Connections', 'Emeritus of Woe', 'Reprieve']`)
et identique pour tous les utilisateurs.

L'objectif est de remplacer cette liste statique par les 4 dernières recherches effectuées par l'utilisateur,
stockées en localStorage.

## Objectif

L'utilisateur voit, sur la page d'accueil, les 4 dernières recherches qu'il a effectuées
(modes "Nom" uniquement), et peut cliquer sur l'une d'elles pour recharger le terme dans le champ de recherche.

## Solution

### Stockage

- Clé localStorage : `tae_recent_searches`.
- Valeur : tableau JSON de chaînes, ex: `["Vampiric Tutor", "Black Market Connections", ...]`.
- Seules les recherches en mode "Nom" sont enregistrées (pas le mode decklist).
- Limite stricte : 4 entrées maximum.

### Gestion des recherches

1. À chaque recherche soumise (bouton "Chercher" ou touche Entrée) en mode "Nom" :
   - Si le terme existe déjà dans la liste : le supprimer.
   - Insérer le terme au début du tableau.
   - Si le tableau dépasse 4 éléments : couper au-delà.
2. À chaque sélection d'un terme dans la liste "récents" (click sur un bouton récent) :
   - Traiter comme une nouvelle recherche : appliquez la même logique (supprimer si existant, insérer en début).
   - Remplir le champ de recherche avec le terme sélectionné.
   - **Ne pas naviguer automatiquement** : l'utilisateur doit cliquer "Chercher" pour lancer la recherche.

### Affichage

- La page lit la liste depuis localStorage au chargement.
- Affiche le tableau (itéré avec `v-for`) sous le champ de recherche, avec l'étiquette "récents".
- Si la liste est vide : ne pas afficher la section (condition `v-if` existante conservée).

### Emplacement

- Modification uniquement du composant `frontend-vue/app/pages/index.vue`.
- Pas de modification de route, d'API, ou de base de données.

### Éléments existants à réutiliser

- `navigateToSearch()` : déjà appelée par l'utilisateur sur "Chercher", pas besoin de changement.
- `selectRecent()` : modifiée pour ajouter la logique de stockage localStorage avant de remplir `q.value`.
- Structure HTML/Template actuelle conservée : même `v-for`, mêmes classes Tailwind, même icône `<AppIcon name="clock" />`.

## Cas d'erreurs

- **localStorage désactivé ou non disponible** (navigateur privé, certains réglages) : pas d'affichage de la liste "récents",
  la page se comporte comme une première visite (liste vide). Aucun message d'erreur affiché.
- **Valeur corrompue** dans localStorage (pas un tableau JSON valide) : la liste est ignorée, affiché comme vide.
- **Même terme plusieurs fois** : géré par la logique de suppression-avant-insertion.
- **Recherche vide** (`q.value === ''`) : ne pas enregistrer dans l'historique.

## Critères d'acceptance

- [ ] L'utilisateur tape "Vampiric Tutor" et clique "Chercher" → revient sur la home → la liste "récents" affiche "Vampiric Tutor".
- [ ] L'utilisateur effectue 4 recherches différentes (A, B, C, D) → la liste affiche [A, B, C, D] dans cet ordre.
- [ ] L'utilisateur effectue une 5e recherche (E) → la liste affiche [E, A, B, C] (D est coupé).
- [ ] L'utilisateur recherche à nouveau A → la liste affiche [A, E, B, C] (A déplacé en haut, D déjà coupé).
- [ ] L'utilisateur clique sur un terme dans "récents" → le terme remplit le champ de recherche (sans naviguer).
- [ ] Le terme cliqué dans "récents" est aussi ajouté comme nouvelle recherche (déplacé en haut si existant).
- [ ] La section "récents" est masquée si l'historique est vide.
- [ ] Clé localStorage `tae_recent_searches` contient un tableau JSON valide après plusieurs recherches.
- [ ] Valeur corrompue dans localStorage → la liste "récents" reste vide, aucun crash.
- [ ] localStorage désactivé → la liste "récents" reste vide, aucun crash.
- [ ] Rechercher avec `q` vide → rien n'est enregistré dans l'historique.
- [ ] Le mode "Decklist" ne crée aucune entrée dans l'historique des recherches récentes.
