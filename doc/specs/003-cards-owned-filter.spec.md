# Spec : Filtre `owned` sur l'endpoint /cards

## Contexte

Aujourd'hui, `GET /cards` filtre systématiquement sur l'utilisateur courant : la requête SQL derrière
`CardPricesViewRepositoryAdapter::get_paginated` (`WHERE cp.user_id = $1`) ne peut renvoyer que les cartes de
l'utilisateur connecté. Il n'existe aucun moyen de voir les cartes possédées par d'autres utilisateurs via cet
endpoint. Une page frontend `find/index.vue` ("Cartes chez les autres joueurs") existe déjà mais n'affiche que des
données mockées, non branchées à l'API.

## Objectif

Faire évoluer `GET /cards` pour qu'il puisse lister le catalogue complet des cartes possédées (par moi ou par
d'autres utilisateurs), et permettre de restreindre ce résultat à mes propres cartes via un query param `owned`.

- `owned=true` : comportement équivalent à l'actuel — uniquement les cartes que je possède.
- `owned` absent ou `owned=false` : toutes les cartes possédées par n'importe quel utilisateur (moi inclus).

## Solution

### Query param

- Ajouter `owned: Option<bool>` à `CollectionParams` (`card_controller.rs`) et à `CollectionQuery`
  (`domain/collection.rs`), suivant le même schéma que les autres filtres existants (`rarity`, `sets`, etc.).
- Absence du param ou `owned=false` → pas de filtre sur `user_id` dans la requête. `owned=true` → filtre
  `cp.user_id = $current_user_id` comme aujourd'hui.

### Résultat multi-utilisateur

- La vue matérialisée `mv_card_prices` contient déjà une ligne par couple (carte, utilisateur possesseur) — pas de
  changement de schéma nécessaire côté vue.
- Une même carte peut donc apparaître plusieurs fois dans la liste paginée si elle est possédée par plusieurs
  utilisateurs différents (une ligne par possesseur). La pagination et le total (`COUNT`) portent sur ces lignes, pas
  sur les cartes uniques.

### Confidentialité des données par ligne

Pour chaque ligne renvoyée :

- Si la ligne correspond à une carte que je possède (`user_id` de la ligne = utilisateur courant) : `quantity`,
  `purchase_price` et `added_at` sont renseignés avec mes valeurs (comme aujourd'hui pour les deux premiers ;
  `added_at` n'est actuellement pas exposé dans la réponse et devient visible dans le cadre de cette évolution).
- Si la ligne correspond à une carte possédée par un autre utilisateur : `quantity`, `purchase_price` et `added_at`
  ne doivent jamais être exposés (absents de la réponse). À la place, exposer le nom d'utilisateur du possesseur
  (`username`, `users.username`, cf. migration `0009_add_users_table.sql`) pour identification.
- `price_guide` (issu des données de marché, indépendant de la possession) reste inchangé dans tous les cas.

### Modifications de contrat (réponse JSON)

- Regrouper `quantity`, `purchase_price` et `added_at` dans un sous-objet `collection_entry`, présent uniquement
  quand la ligne m'appartient (`Option<CollectionEntry>`, `null` si la carte est possédée par un autre utilisateur).
- Ajouter un champ `owner_username: Option<String>` — `null` si la ligne m'appartient (auquel cas c'est
  `collection_entry` qui est renseigné), sinon toujours renseigné (le username est `NOT NULL` en base, cf.
  `migrations/0009_add_users_table.sql`). Un seul des deux champs (`collection_entry` / `owner_username`) est
  renseigné à la fois.
- Mettre à jour le schéma OpenAPI (`doc/openapi.yml`) et les types générés `ts-rs` (`CollectionCard.ts`) en
  conséquence.

### Résolution du username

- Joindre la table `users` sur `collection_entry.user_id` (ou équivalent dans `mv_card_prices`) pour récupérer le
  `username` du possesseur de chaque ligne. La colonne est `NOT NULL` en base : `owner_username` est donc toujours
  renseigné pour une ligne appartenant à un autre utilisateur (jamais `null` dans ce cas). Le type `Option<String>`
  du champ `username` sur `domain::User` (`src/ae/domain/user.rs:8`) ne reflète pas cette contrainte — à corriger
  en `String` non optionnel dans le cadre de cette évolution (ou au moins pour l'usage fait ici).

### Frontend

- `frontend-vue/app/pages/collection/index.vue` doit désormais passer explicitement `owned=true` dans ses appels à
  `/cards`, pour conserver le comportement actuel (uniquement mes cartes) malgré le changement de valeur par défaut
  de l'endpoint.
- Aucune autre page frontend n'est modifiée dans le cadre de cette spec (la page `find/index.vue` reste mockée,
  son branchement à l'API est hors périmètre).

## Cas d'erreurs

- `owned` avec une valeur autre que `true`/`false` (ex: `owned=foo`) : retourner une erreur 400 (format invalide),
  cohérent avec le traitement des autres query params typés de l'endpoint.
- Carte possédée par plusieurs utilisateurs dont moi-même : ma ligne (avec mon `collection_entry`) et les lignes
  des autres utilisateurs (avec leur `owner_username`, sans données financières) apparaissent toutes séparément
  dans le résultat paginé.

## Critères d'acceptance

- [ ] `GET /cards?owned=true` renvoie uniquement les cartes possédées par l'utilisateur authentifié, avec leur
      `collection_entry` (`quantity`/`purchase_price`/`added_at`) renseigné, et `owner_username` à `null`.
- [ ] `GET /cards` (sans `owned`) et `GET /cards?owned=false` renvoient les cartes possédées par tous les
      utilisateurs (union), y compris celles de l'utilisateur courant.
- [ ] Dans le résultat de `GET /cards` (catalogue), une ligne correspondant à une carte possédée par un autre
      utilisateur a `collection_entry` à `null` et `owner_username` toujours renseigné (jamais `null`).
- [ ] Dans le résultat de `GET /cards` (catalogue), une ligne correspondant à une carte possédée par l'utilisateur
      courant a `collection_entry` renseigné (`quantity`/`purchase_price`/`added_at`) et `owner_username` à `null`.
- [ ] Une carte possédée par 3 utilisateurs différents apparaît 3 fois dans le résultat paginé de `GET /cards`
      (sans `owned=true`), une ligne par possesseur.
- [ ] Le total (`total` dans `PaginatedCollectionResponse`) compte les lignes (carte, possesseur), pas les cartes
      uniques.
- [ ] `GET /cards?owned=foo` (valeur non booléenne) retourne une 400.
- [ ] La page collection du frontend continue d'afficher uniquement les cartes de l'utilisateur connecté après la
      modification (appel avec `owned=true`).
- [ ] Les filtres existants (`q`, `rarity`, `sets`, `price_min`, `price_max`, tri) continuent de fonctionner
      identiquement en mode `owned=true` et en mode catalogue.
- [ ] Le schéma OpenAPI et les types TypeScript générés reflètent les nouveaux champs (`owned` en query param,
      `owner_username`, sous-objet `collection_entry` optionnel dans `CollectionCardResponse`).
