# Spec : Nouveau endpoint de recherche publique (`/search/card`)

## Contexte

Le endpoint `GET /collection` assure deux rôles aujourd'hui :

1. Récupérer la collection privée d'un utilisateur (quand `owned=true` ou par défaut).
2. Rechercher/parcourir les cartes de tous les utilisateurs (quand `owned=false`).

Ce mélange de responsabilités pose problème :

- `/collection` expose implicitement des données publiques sans que la sémantique de sa route n'indique clairement cette double fonctionnalité.
- Il n'existe pas de endpoint dédié à la recherche de cartes d'autres utilisateurs, ce qui empêche de créer une page "recherche" ou "découverte" indépendante de la collection personnelle.

## Objectif

Créer un nouveau endpoint `GET /search/card` dédié exclusivement à la recherche de cartes d'autres utilisateurs, avec les mêmes capacités de filtrage, tri et pagination que `/collection`. Le endpoint `/collection` devient strictement privé (plus de mode catalogue public).

La route est préfixée par `/search/card` (plutôt que `/search`) en prévision de futurs endpoints de recherche sur d'autres types de ressources : `/search/deck-list`, `/search/player`.

## Solution

### Nouveau module `search`

- Nouveau module `adapter_in/search`, sur le même modèle que `collection`/`card`/`trade` : fichier `adapter_in/search.rs` déclarant `pub mod controller; pub mod dto; #[cfg(test)] mod tests;`, et un dossier `adapter_in/search/` avec `controller.rs`, `dto.rs`, `tests.rs`.
- `create_search_router()` dans `search/controller.rs` gère le sous-routage de `/search` : `.nest("/card", ...)` avec un `.route("/", get(search_cards))`. Les futurs endpoints (`/search/deck-list`, `/search/player`) s'ajoutent avec un `.nest(...)` supplémentaire dans cette même fonction.
- Montage dans `infrastructure.rs` via `.nest("/search", create_search_router())`.

### Handler de recherche

- Le handler `search_cards` dans `search/controller.rs` parse les mêmes query params que `collection/dto.rs` : `q`, `rarity` (répété), `sets`, `price_min`, `price_max`, `sort_by`, `sort_dir`, `page`, `page_size`.
- Le paramètre `owned` n'existe pas dans `/search/card` : la recherche porte toujours sur les cartes des autres utilisateurs.
- Construction d'une `CollectionQuery` (ou d'un type dédié `SearchQuery` si la séparation domaine est préférable) passant à un use case `SearchCardsUseCase`.
- Le use case réutilise le même repository `CardPricesViewRepository` que `/collection` — la seule différence est l'absence du filtre `user_id` et du filtre `owned`.
- Mapping du résultat vers `PaginatedCollectionResponse` (même format que `/collection`).

### DTOs

- Un nouveau `SearchParams` dans `search/dto.rs` inspiré de `CollectionParams` mais sans le champ `owned`. Réutiliser les mêmes types de tri (`SortByParam`, `SortDirParam`), pagination (`default_page_size`, `max_page_size`), et response (`CollectionCardResponse`, `CollectionEntryResponse`, `PriceGuideResponse`, `PaginatedCollectionResponse`).
- Les types response peuvent être partagés via un module commun ou importés depuis `collection/dto.rs` si la structure est identique.

### Modification de `/collection`

- Suppression du paramètre `owned` du handler `get_collection`.
- La requête devient toujours filtrée sur l'utilisateur connecté (ajout du `user_id` dans la clause WHERE, sans option pour désactiver ce filtre).
- Le tag OpenAPI de `/collection` est mis à jour pour refléter qu'il est exclusivement privé (ex. "Collection (private)").
- `doc/openapi.yml` régénéré.

### Frontend

- Nouvelle page/composable de recherche (ex. `/search`, `useSearchService.ts`) utilisant `GET /search/card` avec les mêmes query params.
- La page collection existante continue d'utiliser `GET /collection` pour les données privées (sans paramètre `owned`).
- Si une page "catalogue public" existait via `/collection?owned=false`, elle doit être migrée vers `GET /search/card`.

## Cas d'erreurs

- `GET /search/card` sans token d'authentification → 401.
- Paramètres invalides (rarity invalide, sort_by invalide, page/page_size hors limites) → 400, mêmes erreurs que `/collection`.
- Aucun résultat → 200 avec une page vide (même comportement que `/collection`).
- `GET /collection?owned=false` (ancienne requête catalogue public) → soit 400 (paramètre inconnu), soit le paramètre est ignoré silencieusement (à décider).

## Critères d'acceptance

- [ ] `GET /search/card?q=goblin` retourne les cartes correspondant à "goblin" dans la collection de tous les utilisateurs (pagination, tri, même format de réponse que `/collection`).
- [ ] `GET /search/card` supporte tous les filtres de `/collection` : `rarity`, `sets`, `price_min`, `price_max`, `sort_by`, `sort_dir`.
- [ ] `GET /search/card` supporte la pagination (`page`, `page_size` avec les mêmes limites).
- [ ] `GET /search/card` est authentifié : 401 sans token.
- [ ] `GET /collection` ne supporte plus le paramètre `owned` (400 si passé, ou ignoré silencieusement).
- [ ] `GET /collection` retourne uniquement les cartes de l'utilisateur authentifié, même sans paramètre `owned`.
- [ ] Le format de réponse de `/search/card` est identique à celui de `/collection` (`PaginatedCollectionResponse`).
- [ ] Le endpoint `/search/card` est documenté dans `doc/openapi.yml` sous un tag `"search"` distinct.
- [ ] Le frontend dispose d'un composable/service (`useSearchService.ts`) appelant `/search/card`.
- [ ] Les tests unitaires du handler `search_cards` couvrent les mêmes cas que `/collection` (réponse nominale, erreurs de validation, 401, pagination, filtres).
- [ ] `mise run lint-backend` et `mise run format` passent sans erreur.
