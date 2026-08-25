# Spec : Icône de set coloré par rareté dans la modale de détail de carte

## Contexte

Dans la modale de détail d'une carte (`frontend-vue/app/components/Card/DetailModal.vue`), le set de la carte est
aujourd'hui affiché uniquement par son code (`card.set_code.toUpperCase()`), suivi du code de rareté brut
(`card.rarity_code`) — ex. `FDN · M`. Aucun nom complet de set ni icône n'est affiché.

La maquette (`maquette/screen_collection.jsx`, fonction `CardDetailModal`) affiche déjà le nom complet du set
accompagné d'un glyph Keyrune neutre (couleur fixe `--ink-2`), mais via des dictionnaires statiques de démonstration
(`SET_NAMES`, `SET_GLYPH`) limités à une poignée de sets — pas une source de données réelle.

Le backend expose déjà, via `GET /collection/stats`, la liste complète des sets présents dans la collection
(`sets: Vec<SetInfoResponse>` avec les champs `code` et `name`). Cette donnée est déjà récupérée côté frontend
(`setList` calculé depuis `statsData.sets` dans `collection/index.vue` et `search/index.vue`) et utilisée par
`CollectionFilters.vue`, mais n'est pas transmise à `CardDetailModal`.

La police Keyrune (symboles de set) est déjà utilisée dans `frontend-vue` : chargée via le CDN jsdelivr
(`nuxt.config.ts`), et affichée dans `CollectionFilters.vue` via les classes `ss`/`ss-{code}` (ex. `ss ss-fdn`). Le
design system (`design-system.instructions.md`) indiquait à tort que ces symboles étaient self-hosted — corrigé pour
refléter l'usage réel du CDN.

Aucune charte de couleurs par rareté n'existe actuellement ni dans le design system, ni dans la maquette, ni dans
`frontend-vue`.

## Objectif

Dans la modale de détail de carte de `frontend-vue`, remplacer l'affichage du code de set par :

- une icône du set (police Keyrune), colorée selon la rareté de la carte affichée ;
- le nom complet du set (ex. "Foundations" plutôt que "FDN").

## Solution

### Données

- Aucune modification backend : réutiliser tel quel `GET /collection/stats` (`sets[].code`, `sets[].name`).
- Transmettre la prop `set-list` (déjà calculée en page, cf. `setList` dans `collection/index.vue` et
  `search/index.vue`) au composant `CardDetailModal`, qui ne la reçoit pas actuellement.
- Résoudre le nom complet du set à partir de `card.set_code` en cherchant dans `set-list`, sur le même principe que
  `setNameByCode` dans `CollectionFilters.vue`. Cette fonction de résolution devient partagée/dupliquée selon ce qui
  est le plus cohérent avec le composant — à trancher au plan.

### Icône de set

- Réutiliser l'intégration Keyrune déjà en place (CDN jsdelivr, classes `ss`/`ss-{code}`), sur le même principe que
  `CollectionFilters.vue`. Aucun ajout de police nécessaire.
- Le glyph affiché correspond au `set_code` de la carte.

### Couleur par rareté

Convention MTG classique, appliquée à la couleur de l'icône du set (pas au texte). Keyrune fournit nativement des
classes modificatrices de couleur par rareté (`ss-common`, `ss-uncommon`, `ss-rare`, `ss-mythic`), envisagées un
temps en priorité pour ces 4 rangs — écartées au plan : ces classes portent des couleurs fixes non thémables, alors
que l'app doit s'adapter au thème clair/sombre (cf. `main.css`, tokens redéfinis dans `:root:not(.dark)`). 5 tokens
CSS custom homogènes (`--rarity-*`) sont utilisés à la place pour les 5 rangs, y compris `S` qui n'a de toute façon
pas d'équivalent natif Keyrune.

| Rareté (`rarity_code`) | Couleur           | Source                                                                          |
| ---------------------- | ----------------- | ------------------------------------------------------------------------------- |
| `C` (Common)           | Noir / gris foncé | Token custom `--rarity-common`                                                  |
| `U` (Uncommon)         | Argent            | Token custom `--rarity-uncommon`                                                |
| `R` (Rare)             | Or                | Token custom `--rarity-rare`                                                    |
| `M` (Mythic)           | Orange-rouge      | Token custom `--rarity-mythic`                                                  |
| `S` (Special)          | Rose / magenta    | Token custom `--rarity-special` (rareté custom, cf. [[014-add-special-rarity]]) |

### Affichage

- L'icône colorée est affichée en préfixe de la ligne de métadonnées de la carte (remplace la position actuelle du
  code de set).
- Le texte passe de `SET_CODE · RARITY_CODE` à `Nom complet du set` seul : le code de rareté textuel est retiré, la
  rareté étant désormais portée par la couleur de l'icône.

## Cas d'erreurs

- `card.set_code` absent de `set-list` (donnée non trouvée) : afficher le code de set brut en majuscules comme
  fallback (comportement actuel), sans icône colorée.
- `card.rarity_code` ne correspond à aucune des 5 valeurs connues (C/U/R/M/S) : afficher l'icône du set sans couleur
  de rareté (couleur neutre `--ink-2`, comportement actuel du glyph dans la maquette).
- `set-list` vide ou non chargée (ex. endpoint `/collection/stats` en erreur) : la modale reste fonctionnelle,
  affiche le code de set brut comme fallback, sans bloquer l'ouverture de la modale.

## Critères d'acceptance

- [ ] Given une carte dont le `set_code` est présent dans `set-list`, when la modale de détail s'ouvre, then le nom
      complet du set est affiché (pas le code).
- [ ] Given une carte de rareté Common, when la modale s'ouvre, then l'icône du set est affichée en noir/gris foncé.
- [ ] Given une carte de rareté Uncommon, when la modale s'ouvre, then l'icône du set est affichée en argent.
- [ ] Given une carte de rareté Rare, when la modale s'ouvre, then l'icône du set est affichée en or.
- [ ] Given une carte de rareté Mythic, when la modale s'ouvre, then l'icône du set est affichée en orange-rouge.
- [ ] Given une carte de rareté Special, when la modale s'ouvre, then l'icône du set est affichée en rose/magenta.
- [ ] Given une carte dont le `set_code` est absent de `set-list`, when la modale s'ouvre, then le code de set brut
      en majuscules est affiché à la place du nom complet, sans erreur ni crash.
- [ ] Given une carte dont le `set_code` est absent de `set-list`, when la modale s'ouvre, then l'icône du set est
      affichée dans une couleur neutre, quelle que soit la rareté de la carte (pas de couleur de rareté appliquée à
      un set inconnu).
- [ ] Given une carte dont le `rarity_code` ne correspond à aucune des 5 valeurs connues, when la modale s'ouvre,
      then l'icône du set est affichée dans une couleur neutre (pas de couleur de rareté appliquée).
- [ ] `CardDetailModal` reçoit bien la prop `set-list` depuis `collection/index.vue` et `search/index.vue`.
- [ ] `design-system.instructions.md` ne décrit plus les symboles de set Keyrune comme self-hosted, et mentionne le
      chargement via le CDN jsdelivr.
- [ ] `mise run lint-frontend` passe sans erreur.
- [ ] `mise run format` passe sans erreur.
