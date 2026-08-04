# Spec : Endpoint autocomplete utilisateur

## Contexte

Le `PlayerPicker.vue` de la page d'accueil utilise actuellement des données mockées (`MOCK_PLAYERS`) pour le dropdown de recherche de joueurs. L'utilisateur doit pouvoir taper une portion de username et voir une liste de suggestions en temps réel. L'objectif est de remplacer ce mock par un vrai endpoint API.

L'endpoint doit être public (non authentifié) car il est utilisé sur la page d'accueil avant toute connexion.

## Objectif

Créer `GET /autocomplete/user?q={{username}}` qui retourne une liste de joueurs correspondant à la query, avec :

- `username` : le nom du joueur
- `note` : valeur hardcodée à `5` (champ purement UI, pas stocké en base)
- `card_count` : nombre de cartes possédées par l'utilisateur (calculé depuis `collection_entry`)

## Solution

### Endpoint

`GET /autocomplete/user?q={{username}}` — non authentifié, response JSON.

Le query param `q` est optionnel. Si absent ou vide, retourne une liste vide.

### Indexation

- Ajouter une migration créant un index **GIN** sur la colonne `users.username` via `pg_trgm`, sur le modèle de
  l'index existant sur `mv_card_prices.name` (voir `0003_add_pgtrm_extension.sql`).
- Extension `pg_trgm` : déjà installée (migration `0003_add_pgtrm_extension.sql`), pas de nouvelle migration à créer.

### Matching

- **Fuzzy trigram** : même pattern que le filtrage existant sur `mv_card_prices.name`
  (`username ILIKE '%' || q || '%' OR q <% username`) — combinaison substring + opérateur `word_similarity`
  de `pg_trgm` (seuil implicite pg_trgm par défaut, pas de seuil explicite à coder).
- **Scoring** : trier les résultats par `word_similarity(q, username) DESC` pour afficher en premier les matches
  les plus proches, comme pour la recherche de cartes.
- **Insensible à la casse** : `ILIKE` est nativement insensible à la casse ; `pg_trgm` doit être appliqué sur des
  valeurs normalisées en minuscule (`LOWER(username)`, `LOWER(q)`) pour que l'opérateur `<%` le soit aussi.
- Limité à **10 résultats maximum**.

### Payload

Chaque entrée de réponse contient :

```json
{
  "username": "alice",
  "note": 5,
  "card_count": 42
}
```

- `note` : toujours `5` (hardcodé)
- `card_count` : `SUM(quantity)` sur `collection_entry` pour l'utilisateur — total de cartes possédées
  (même sémantique que `CollectionStats.total_cards`), `0` si aucun enregistrement

### Méthode de récupération

- Requête SQL utilisant `pg_trgm`, sur le même modèle que `build_filter_clause` existant :
  - `WHERE username ILIKE '%' || q || '%' OR q <% username`
  - `ORDER BY word_similarity(q, username) DESC`
  - `LIMIT 10`
- Pour `card_count` : `SUM(quantity)` via jointure `LEFT JOIN collection_entry` group by user, ou sous-requête
  corrélée dans le `SELECT`.
- Exemple : `SELECT u.id, u.username, (SELECT COALESCE(SUM(quantity), 0) FROM collection_entry ce WHERE ce.user_id = u.id) AS card_count FROM users u WHERE ...`

### Architecture

Hexagonale, même pattern que les autres endpoints publics :

- **Handler** (adapter_in) : extrait `q`, délègue au use case
- **Use case** (service) : délègue au repository
- **Repository port** : interface avec méthode `autocomplete_users(query: Option<String>)`
- **Repository adapter** : requête SQL utilisant `ILIKE` + l'opérateur `<%` et `word_similarity()` de `pg_trgm`

### Frontend

- Adapter `PlayerPicker.vue` pour utiliser l'endpoint au lieu du mock
- Ne conserver que `username`, `note`, `card_count` du payload
- Renommer le champ `handle` en `username` dans l'interface `Player` (et partout où il est utilisé :
  `PlayerAvatar`, navigation, `index.vue`)
- Supprimer le rendu de `initials` calculé en dur ; le générer à la volée depuis le `username`
  (2 premières lettres majuscules)
- Supprimer le rendu de `trades` (champ `trades` du `Player` interface retiré)
- Garder le `rating` = affichage de `note` avec une étoile (ex: "★★★★★" pour 5)
- Ajuster le `goToPlayer` pour naviguer vers `/search?player=${username}`
- **Recherches récentes** : réutiliser le même mécanisme que `useRecentSearches()` (localStorage) pour
  stocker les derniers usernames sélectionnés ; croiser côté front avec les résultats de l'API autocomplete
  pour reconstituer les sections "récents" / "autres joueurs" (le flag `recent` n'est plus fourni par l'API)

### Hors scope

- Pagination (max 10 résultats fixes)
- Trie des résultats (ordre de la base, par défaut)
- Cache des résultats
- Suggestions de spelling (typo tolerance)
- Compteur de trades (champ `trades` retiré du frontend)

## Cas d'erreurs

- **Query vide** : retourne une liste vide `[]` (200 OK)
- **Pas de match** : retourne une liste vide `[]` (200 OK)
- **Erreur de base de données** : retourne 500 avec message d'erreur
- **Paramètre `q` manquant** : comportement identique à query vide → liste vide

## Critères d'acceptance

- [ ] `GET /autocomplete/user?q=ali` retourne 200 avec des utilisateurs similaires à "ali" via trigramme
      (ex: "alice", "mallory")
- [ ] `GET /autocomplete/user?q=al` retourne "alice" en premier (meilleur score `word_similarity`)
- [ ] `GET /autocomplete/user?q=ali` est **insensible à la casse** (retourne "Alice" pour "ali", "ALI", "Ali")
- [ ] `GET /autocomplete/user?q=xyz` (aucun match ILIKE ni trigramme) retourne une liste vide `[]`
- [ ] `GET /autocomplete/user` (sans paramètre) retourne une liste vide `[]`
- [ ] `GET /autocomplete/user` (avec paramètre vide `?q=`) retourne une liste vide `[]`
- [ ] La réponse contient au maximum 10 résultats
- [ ] Chaque résultat contient les champs `username` (string), `note` (number = 5), `card_count` (number)
- [ ] `note` vaut toujours `5` pour chaque résultat
- [ ] `card_count` reflète la somme des quantités (`SUM(quantity)`) dans `collection_entry` pour l'utilisateur,
      pas le nombre de lignes distinctes
- [ ] `card_count` vaut `0` pour un utilisateur sans carte
- [ ] L'endpoint est **public** (accessible sans token Clerk)
- [ ] Le `PlayerPicker.vue` utilise l'endpoint au lieu du mock
- [ ] L'interface `Player` côté front utilise `username` (plus `handle`)
- [ ] Le `PlayerPicker.vue` ne rend plus `trades` ni ne calcule `initials` depuis un champ externe
- [ ] Le `PlayerPicker.vue` génère `initials` à partir du `username` (2 premières lettres majuscules)
- [ ] Le `PlayerPicker.vue` affiche `note` avec une étoile (★)
- [ ] Les recherches de joueurs récentes sont stockées en localStorage et affichées dans une section
      "récents" distincte des autres résultats de l'API
- [ ] Le clic sur un joueur navigue vers `/search?player=${username}`
- [ ] `mise run lint-backend` passe sans erreur
- [ ] `mise run lint-frontend` passe sans erreur
