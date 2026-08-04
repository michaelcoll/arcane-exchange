# Spec : Extraction du endpoint de collection (`/cards` → `/collection`)

## Contexte

Le endpoint `GET /cards` retourne la collection paginée d'un joueur (ou le catalogue complet filtré, selon le
paramètre `owned`). Son nom ne correspond pas à son usage réel : il ne renvoie pas "toutes les cartes" mais la
collection/le catalogue consultable par un joueur. Il vit aujourd'hui dans le controller `card`
(`adapter_in/card/controller.rs`), aux côtés d'endpoints qui, eux, concernent réellement des cartes prises
individuellement : `/cards/import`, `/cards/card-info`, `/cards/price-history`, `/cards/stats`,
`/cards/{scryfall_id}/price-history`.

À terme, `/cards/import`, `/cards/price-history` et `/cards/stats` seront extraits eux aussi vers un module dédié à
la collection. Cette spec ne couvre que l'extraction de `GET /cards` — les autres endpoints restent inchangés dans
le controller `card`.

## Objectif

Déplacer `GET /cards` vers `GET /collection`, dans un nouveau module `collection` séparé (nouveau controller,
nouveau routeur), sans toucher au controller `card` existant ni aux autres endpoints qu'il expose. L'ancienne route
`GET /cards` doit disparaître (404) une fois l'extraction faite — pas d'alias de compatibilité.

## Solution

### Nouveau module `collection`

- Nouveau module `adapter_in/collection`, sur le même modèle que `card`/`trade`/`maintenance` : fichier
  `adapter_in/collection.rs` déclarant `pub mod controller; pub mod dto; #[cfg(test)] mod tests;`, et un dossier
  `adapter_in/collection/` avec `controller.rs`, `dto.rs`, `tests.rs`.
- `create_collection_router()` dans `collection/controller.rs`, sur le modèle de `create_card_router()` : un seul
  `.route("/", get(get_collection))`.
- Montage dans `infrastructure.rs` via `.nest("/collection", create_collection_router())`, en plus du
  `.nest("/cards", create_card_router())` existant (qui reste, pour les autres endpoints du controller `card`).

### Déplacement du handler

- Le handler `get_collection` (actuellement `card/controller.rs`) est déplacé tel quel vers
  `collection/controller.rs` : même logique de parsing des query params (`rarity`, `sets`, tri, pagination), même
  construction de `CollectionQuery`, même appel à `state.get_collection_use_case.get_collection(...)`. Aucune
  logique métier ne change — seul l'emplacement du code et la route changent.
- Le handler n'est plus présent dans `card/controller.rs` / `create_card_router()`.

### Déplacement des DTOs

- Les DTOs utilisés exclusivement par cet endpoint sont déplacés de `card/dto.rs` vers un nouveau `collection/dto.rs` :
  `CollectionParams`, `SortByParam`, `SortDirParam` (+ leurs `impl From<...>` vers les types domaine
  `CollectionSortField`/`SortDirection`), `default_page_size`, `max_page_size`, `CollectionCardResponse`,
  `CollectionEntryResponse`, `PriceGuideResponse`, `PaginatedCollectionResponse` (+ `impl From<Card> for
CollectionCardResponse`).
- Les DTOs encore utilisés par les endpoints restant dans `card` (`MessageResponse`, `PriceHistoryParams`,
  `PriceHistoryEntryResponse`, `CollectionStatsResponse`, `SetInfoResponse`) restent dans `card/dto.rs`.

### AppState / use cases

- Aucun changement côté `application/` : le nouveau handler réutilise le même champ `state.get_collection_use_case`
  déjà présent dans `AppState`. Pas de duplication de service ni de branchement supplémentaire.

### Documentation API

- Nouveau tag OpenAPI `"collection"` (ex. description : "Player's collection (authentication required)"), déclaré
  dans `openapi.rs` (`tags(...)`).
- `#[utoipa::path]` du handler `get_collection` mis à jour : `path = "/collection"`, `tag = "collection"`.
- `openapi.rs` : `super::card::controller::get_collection` remplacé par
  `super::collection::controller::get_collection` dans `paths(...)` ; les schémas déplacés (`CollectionCardResponse`,
  `PaginatedCollectionResponse`, `SortByParam`, `SortDirParam`, `PriceGuideResponse`, `CollectionEntryResponse`) sont
  importés depuis `super::collection::dto::` au lieu de `super::card::dto::`.
- `doc/openapi.yml` régénéré pour refléter le nouveau chemin et le nouveau tag.

### Frontend

- `frontend-vue/app/composables/useCardsService.ts` : `getCollection` doit appeler `/collection` au lieu de
  `/cards`. Mêmes query params, même réponse (le binding `PaginatedCollection.ts` ne change pas de forme). Renommer
  le composable lui-même (ex. `useCollectionService`) n'est pas dans le périmètre de cette spec.
- `getCollection2` (même appel que `getCollection` mais sans passer par `useAsyncData`) n'a plus aucun appelant dans
  le code — à supprimer avec son export, au lieu d'être migrée vers `/collection`.

## Cas d'erreurs

- `GET /cards` (ancienne route) → 404, la route n'existe plus.
- `GET /collection` sans token d'authentification → 401 (comportement inchangé par rapport à l'ancien `GET /cards`).
- Les erreurs métier existantes (ex. code de rareté invalide dans `rarity` → 400) sont inchangées, seulement
  déplacées avec le handler.
- Les autres endpoints du controller `card` (`/cards/import`, `/cards/card-info`, `/cards/price-history`,
  `/cards/stats`, `/cards/{scryfall_id}/price-history`) ne doivent présenter aucune régression.

## Critères d'acceptance

- [ ] `GET /collection` retourne exactement la même réponse (structure, pagination, tri, filtres) que l'ancien
      `GET /cards`, pour les mêmes paramètres.
- [ ] `GET /collection` est authentifié : 401 sans token.
- [ ] `GET /cards` retourne 404 une fois l'extraction faite.
- [ ] `POST /cards/import`, `POST /cards/card-info`, `GET /cards/price-history`, `GET /cards/stats`,
      `GET /cards/{scryfall_id}/price-history` continuent de fonctionner sans changement de comportement.
- [ ] Le endpoint est documenté dans `doc/openapi.yml` sous le tag `"collection"`, distinct du tag `"cards"`.
- [ ] Le frontend (`useCardsService.ts`) appelle `/collection` pour récupérer la collection paginée ; la page
      collection continue de fonctionner (chargement, pagination, tri, filtres, recherche, `owned`).
- [ ] `getCollection2` (fonction morte, aucun appelant) est supprimée de `useCardsService.ts`.
- [ ] Les tests unitaires du handler (déplacés vers `collection/tests.rs`) couvrent les mêmes cas que l'ancien
      `card/tests.rs` pour cet endpoint (réponse nominale, erreurs de validation, 401).
- [ ] `mise run lint-backend` et `mise run format` passent sans erreur.
