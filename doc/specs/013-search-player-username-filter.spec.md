# Spec : Filtre `player_username` sur `GET /search/card`

## Contexte

Le endpoint `GET /search/card` (voir `008-search-endpoint.spec.md`) recherche des cartes possédées par
n'importe quel utilisateur, tous filtres confondus (`q`, `rarity`, `sets`, `price_min`, `price_max`). Depuis
`009-search-owner-count.spec.md`, les résultats sont regroupés par carte unique et exposent `owner_count`
(nombre de possesseurs distincts) au lieu d'un `owner_username` par ligne.

Il n'existe aujourd'hui aucun moyen de restreindre cette recherche aux cartes possédées par un joueur précis.
Côté frontend, le `PlayerPicker.vue` (voir `012-user-autocomplete-endpoint.spec.md`) navigue déjà vers
`/search?player=${username}` au clic sur un joueur, mais la page `/search` n'exploite pas ce paramètre.

## Objectif

Ajouter un query param `player_username` à `GET /search/card` qui restreint les résultats aux cartes
possédées par l'utilisateur dont le username correspond **exactement** (insensible à la casse) à la valeur
fournie. Aucun matching flou ou partiel n'est appliqué (contrairement à `q` ou à l'autocomplete username).

Si le `player_username` fourni ne correspond à aucun utilisateur existant, le endpoint retourne une liste
vide plutôt qu'une erreur.

## Solution

### Query param

- Nouveau champ `player_username: Option<String>` dans `SearchParams` (`search/dto.rs`), optionnel.
- Absent ou chaîne vide → comportement inchangé, aucun filtre appliqué (même sémantique que `q` aujourd'hui).
- Combinable en ET logique avec les filtres existants (`q`, `rarity`, `sets`, `price_min`, `price_max`).
- Ce paramètre n'existe que sur `/search/card` ; `GET /collection` n'est pas concerné (déjà filtré sur
  l'utilisateur authentifié).

### Matching

- Correspondance exacte sur `users.username`, insensible à la casse (`LOWER(username) = LOWER($player_username)`).
- Pas d'usage de `ILIKE '%...%'` ni de `pg_trgm` ici : seule une correspondance exacte (à la casse près)
  sélectionne un utilisateur.
- Le username n'a pas de contrainte d'unicité en base (`0009_add_users_table.sql`) ; ce cas est hors scope,
  l'unicité en pratique (gérée par Clerk) est supposée.

### Repository

- Le mode de recherche publique (`search_paginated`, `card_prices_view_repository_adapter.rs`) doit pouvoir
  restreindre la sélection à un utilisateur donné avant l'agrégation par carte, en plus des filtres existants
  (`build_filter_clause`). Cela nécessite de réintroduire une résolution du `username` vers un `user_id`
  (jointure ou sous-requête sur `users`), uniquement lorsque `player_username` est fourni — le mode sans ce
  filtre continue de ne pas joindre `users` (cf. `009-search-owner-count.spec.md`).
- Quand `player_username` est fourni, `owner_count` vaut toujours `1` pour chaque carte retournée : il
  reflète le résultat filtré (les cartes de ce joueur), pas le nombre réel de possesseurs tous joueurs
  confondus.

### Frontend

- La page `/search` (`pages/search/index.vue`) lit le paramètre `player` de l'URL (déjà poussé par
  `PlayerPicker.goToPlayer`, cf. spec 012) et l'envoie comme `player_username` à `GET /search/card`, sur le
  même modèle que la lecture actuelle de `q`/`mode` dans `onMounted`.
- Le binding généré `SearchParams` expose `player_username`.
- Le badge de nombre de possesseurs (`ownerCount` dans `Card/Cell.vue`, et tout autre endroit l'affichant,
  ex. `Card/DetailModal.vue`) n'est plus affiché quand sa valeur vaut `1` — que ce soit parce que la carte n'a
  réellement qu'un possesseur ou parce que le résultat est filtré par `player_username`. Il reste affiché pour
  toute valeur ≥ 2.

## Cas d'erreurs

- `player_username` ne correspondant à aucun utilisateur → 200 avec une liste vide, `total = 0` (même
  comportement qu'une recherche `q` sans résultat).
- `player_username` vide (`?player_username=`) → traité comme absent, aucun filtre appliqué.
- `player_username` combiné à d'autres filtres qui n'ont aucune carte en commun (ex. joueur existant mais ne
  possédant aucune carte correspondant à `q`) → liste vide, `total = 0`.
- `GET /search/card` sans token d'authentification → 401 (comportement existant, inchangé).

## Critères d'acceptance

- [ ] `GET /search/card?player_username=alice` retourne uniquement les cartes possédées par l'utilisateur de
      username exact "alice".
- [ ] `GET /search/card?player_username=Alice` et `GET /search/card?player_username=alice` retournent le même
      résultat (insensible à la casse).
- [ ] `GET /search/card?player_username=ali` (sous-chaîne de "alice", pas de match exact) retourne une liste
      vide.
- [ ] `GET /search/card?player_username=unknown-user` (username inexistant) retourne 200 avec une liste vide
      et `total = 0`.
- [ ] `GET /search/card?player_username=` (vide) ou sans le paramètre → comportement inchangé par rapport à
      aujourd'hui.
- [ ] `player_username` est combinable avec `q`, `rarity`, `sets`, `price_min`, `price_max` (ET logique).
- [ ] Chaque carte retournée par une recherche filtrée par `player_username` a `owner_count = 1`.
- [ ] `GET /search/card` reste authentifié : 401 sans token, y compris avec `player_username` fourni.
- [ ] Le frontend n'affiche plus le badge `owner_count` quand sa valeur vaut `1`, sur la page de recherche et
      dans la modale de détail.
- [ ] La page `/search` lit le paramètre `player` de l'URL et l'utilise comme `player_username` dans la
      requête `GET /search/card` (sur le modèle de la lecture existante de `q`).
- [ ] `doc/openapi.yml` documente `player_username` sur `/search/card`.
- [ ] Les tests unitaires (repository + handler) couvrent : match exact, insensibilité à la casse, username
      inconnu (liste vide), absence de matching partiel, combinaison avec d'autres filtres, `owner_count = 1`.
- [ ] `mise run lint-backend`, `mise run lint-frontend` et `mise run format` passent sans erreur.
