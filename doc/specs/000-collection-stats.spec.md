# Spec : Endpoint de stats de collection

## Contexte

- `setList` et `setCounts` dans la page collection étaient calculés côté front à partir des cartes chargées (pagination) — donc incomplets si toutes les pages ne sont pas chargées
- Le compteur `1 248 cartes · X uniques` avait son premier chiffre hardcodé
- Le filtre de plage de prix avait des bornes statiques (0 – 150 €)

## Objectif

Créer `GET /cards/stats` (authentifié, user courant) qui retourne les stats globales de la collection, et câbler ces données dans la page collection pour alimenter le message de comptage et les filtres.

---

## Décisions

### Endpoint

`GET /cards/stats` — authentifié, calculé à chaque appel (pas de cache).

Champs retournés :

- `total_cards` : somme des quantités de toutes les entrées de l'utilisateur
- `unique_cards` : nombre de lignes distinctes dans `collection_entry` (clé PK = set_code + collector_number + language_code + foil)
- `price_trend_min` / `price_trend_max` : min/max du champ `trend` parmi les prix disponibles, en centimes — `null` si aucun prix
- `sets` : liste des sets distincts de la collection avec `code` et `name`, triés par nom ASC

### Architecture

Hexagonale, même pattern que les autres use cases : handler → use case trait → service → repository port → adapter.

### Domaine

`SetName` (partagé avec `Card`) utilisé directement — pas de struct dédié nécessaire.

### Frontend

- `setList` alimenté depuis les stats (liste complète) plutôt que dérivé des cartes paginées
- `setCounts` supprimé — le count par set est porté directement dans `SetInfo.card_count`
- Le composant `CollectionFilters` reçoit `SetInfo[]` au lieu de `string[]`
- Le message de comptage utilise `total_cards` / `unique_cards` depuis les stats

---

## Hors scope

- Cache des stats
- Filtrage serveur par set ou plage de prix (filtres restent côté client)
- Stats admin globales (déjà dans `StatsUseCase` existant — non modifié)
- Pagination des sets

---

## Critères d'acceptance

- [ ] `GET /cards/stats` retourne 200 avec les champs attendus pour l'utilisateur authentifié
- [ ] `GET /cards/stats` retourne 401 sans token
- [ ] `total_cards` reflète la somme des quantités (pas le nombre de lignes)
- [ ] `unique_cards` reflète le nombre de lignes distinctes dans `collection_entry`
- [ ] `sets` contient tous les sets de la collection (pas seulement la page courante)
- [ ] `price_trend_min` / `price_trend_max` sont `null` si aucune carte n'a de prix trend
- [ ] Le message en haut de la page collection affiche les vraies valeurs
- [ ] Le filtre par set liste tous les sets de la collection avec leur nom complet
- [ ] Les tests unitaires du handler couvrent : réponse normale, collection vide, erreur repository
