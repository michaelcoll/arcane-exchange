# Spec : Centralisation des règles de pagination

## Contexte

Quatre endpoints sont paginés aujourd'hui (`GET /card/offers`, `GET /collection`, `GET /search/card`,
`GET /trades`). Chacun déclare ses propres paramètres `page` et `page_size` dans son DTO de query, avec ses propres
valeurs par défaut et sa propre logique de bornage écrite à la main dans le contrôleur. Le résultat est incohérent :

- `page_size` par défaut vaut 6 sur `/card/offers` et 20 sur les trois autres.
- `/card/offers` et `/trades` appliquent `clamp(1, MAX_PAGE_SIZE)` sur `page_size` et plafonnent `page` à
  `MAX_PAGE_NUMBER` ; `/collection` et `/search/card` appliquent seulement `min(MAX_PAGE_SIZE)` et ne bornent pas
  `page` du tout.
- Le plafond global `MAX_PAGE_NUMBER = 10` est inadapté : sur `/collection`, il rendrait inaccessible tout ce qui
  dépasse quelques centaines de cartes, alors que sur `/card/offers` seules les premières offres ont un intérêt.
- Toutes les valeurs hors bornes sont aujourd'hui silencieusement corrigées : un client qui demande
  `page_size=10000` reçoit 100 éléments et un `200 OK`, sans jamais savoir que sa requête était invalide.

Le domain contient déjà des types de requête (`CollectionQuery`, `TradeListQuery`) et de résultat paginé
(`PaginatedCollection`, `PaginatedTrades`, `PaginatedCardOffers` — trois structures au schéma identique), mais aucune
règle de pagination n'y vit.

## Objectif

Regrouper toutes les règles métier portant sur le couple `page` / `page_size` dans une structure unique du domain.
Cette structure est le seul endroit qui sait ce qu'est une pagination valide : elle refuse les demandes aberrantes au
lieu de les corriger en silence, et elle fournit aux repositories les valeurs dérivées dont ils ont besoin. Un endpoint
paginé ne doit plus pouvoir se tromper : il ne manipule plus `page` et `page_size` directement.

## Solution

### Structure de pagination dans le domain

Une structure du domain porte le couple `page` / `page_size` validé. Elle ne peut être construite qu'en passant par sa
validation — il ne doit pas exister de chemin permettant d'obtenir une pagination invalide. Elle expose les valeurs
dérivées nécessaires aux repositories (offset et limite), de façon à ce que le calcul `page * page_size` disparaisse
des adapters.

La pagination reste **offset-based** et **0-based** : `page=0` est la première page, comme aujourd'hui. Aucun
changement de contrat pour le front sur ce point.

### Règles de validation

- `page_size` doit être compris entre 1 et un maximum, bornes incluses.
- `page` doit être supérieur ou égal à 0.
- La profondeur de pagination est bornée par l'**offset** (`page * page_size`), et non par le numéro de page. Une
  petite `page_size` permet donc d'aller plus loin en numéro de page qu'une grosse, à profondeur équivalente.
- Le maximum de `page_size` et le maximum d'offset sont des paramètres de la construction, pas des valeurs figées
  dans le domain : c'est l'appelant qui impose ses limites.

### Limites par endpoint

Les limites (offset maximum, et `page_size` maximum si un endpoint doit être plus restrictif) sont des constantes en
dur définies dans la couche **application**, au plus près du service qui gère l'endpoint concerné. Elles ne passent
plus par la configuration d'environnement.

Chaque endpoint fixe sa propre profondeur selon son usage :

- `/card/offers` : seules les premières offres ont un intérêt, la profondeur autorisée est faible.
- `/collection`, `/search/card` : un utilisateur doit pouvoir parcourir l'intégralité de sa collection ou d'un
  résultat de recherche, la profondeur autorisée est large.
- `/trades` : profondeur intermédiaire.

Les variables d'environnement `MAX_PAGE_SIZE` et `MAX_PAGE_NUMBER` et les champs correspondants dans l'état
applicatif sont supprimés, ainsi que le bornage manuel présent dans les contrôleurs.

### Valeurs par défaut

Une seule valeur par défaut pour tous les endpoints : `page = 0`, `page_size = 20`. Les défauts spécifiques par
endpoint (notamment `page_size = 6` sur `/card/offers`) disparaissent — c'est au front de demander explicitement la
taille de page dont il a besoin. Le front devra donc être ajusté là où il s'appuyait sur un défaut différent de 20.

### Résultat paginé

Les trois structures de résultat paginé du domain (`PaginatedCollection`, `PaginatedTrades`, `PaginatedCardOffers`),
qui portent le même schéma `items` / `total` / `page` / `page_size`, sont remplacées par un seul type générique du
domain. Ce type conserve la pagination validée qui a produit le résultat, ainsi que le total d'éléments. Les DTO de
réponse HTTP existants et leur forme JSON restent inchangés — c'est un refactoring interne, l'API publique ne bouge
pas.

### Remontée des erreurs

Une pagination invalide est une erreur fonctionnelle, remontée via `FunctionalError` et traduite en **400 Bad
Request** par le mapping HTTP existant, au format de réponse d'erreur déjà en place. Le message doit indiquer quel
paramètre est en cause et quelle est la borne attendue.

## Cas d'erreurs

- `page_size = 0` → 400, message précisant que la taille de page minimale est 1.
- `page_size` supérieur au maximum de l'endpoint → 400, message précisant le maximum autorisé.
- `page` ou `page_size` négatif ou non numérique → 400. Ces valeurs sont rejetées au désérialisage des query params,
  avant même d'atteindre la structure de pagination.
- `page * page_size` supérieur à l'offset maximum de l'endpoint → 400, message précisant que la profondeur de
  pagination demandée est trop grande.
- Une page valide mais au-delà du nombre total de résultats (ex : `page = 5` sur 12 résultats avec `page_size = 20`)
  n'est **pas** une erreur : la réponse est `200 OK` avec une liste vide et le `total` réel.
- Un `total` nul renvoie `200 OK`, liste vide, sans erreur.
- Le dépassement d'offset est évalué sur les valeurs demandées, indépendamment du nombre réel de résultats : une
  requête trop profonde est refusée même si la table est vide.

## Critères d'acceptance

- [ ] Il existe une structure unique dans `domain` portant le couple `page` / `page_size` validé, et il n'est pas
      possible d'en obtenir une instance sans passer par sa validation.
- [ ] Cette structure expose l'offset et la limite dérivés ; plus aucun calcul `page * page_size` ne subsiste dans les
      adapters de repository.
- [ ] Given `page_size = 0`, when un endpoint paginé est appelé, then la réponse est 400 et le message mentionne la
      taille de page minimale.
- [ ] Given `page_size` supérieur au maximum de l'endpoint, when l'endpoint est appelé, then la réponse est 400 et le
      message mentionne le maximum autorisé (plus aucun clamp silencieux ne renvoie 200).
- [ ] Given `page = -1` ou `page_size = abc`, when l'endpoint est appelé, then la réponse est 400.
- [ ] Given `page * page_size` dépassant l'offset maximum de l'endpoint, when l'endpoint est appelé, then la réponse
      est 400 et le message mentionne la profondeur de pagination.
- [ ] Given une `page_size` faible et une `page` élevée dont l'offset reste sous la limite, when l'endpoint est
      appelé, then la réponse est 200 — le numéro de page seul ne provoque jamais de rejet.
- [ ] Given une page valide au-delà du dernier résultat, when l'endpoint est appelé, then la réponse est 200, `items`
      est vide et `total` reflète le nombre réel de résultats.
- [ ] Given aucun paramètre de pagination fourni, when un endpoint paginé est appelé, then `page = 0` et
      `page_size = 20`, pour les quatre endpoints paginés.
- [ ] Les quatre endpoints paginés (`/card/offers`, `/collection`, `/search/card`, `/trades`) partagent exactement le
      même comportement de validation ; aucun bornage manuel de `page` ou `page_size` ne subsiste dans les
      contrôleurs.
- [ ] Chaque endpoint définit sa limite d'offset sous forme de constante dans la couche `application`, près de son
      service ; l'offset maximum de `/card/offers` est plus faible que celui de `/collection` et `/search/card`.
- [ ] Les variables d'environnement `MAX_PAGE_SIZE` et `MAX_PAGE_NUMBER` ne sont plus lues nulle part, et les champs
      correspondants ont disparu de l'état applicatif et de la documentation.
- [ ] Les trois structures `PaginatedCollection`, `PaginatedTrades` et `PaginatedCardOffers` sont remplacées par un
      seul type générique du domain.
- [ ] La forme JSON des réponses paginées (`items`, `total`, `page`, `page_size`) est inchangée pour les quatre
      endpoints.
- [ ] Les tests de clamping existants (`card/tests.rs`, `collection/tests.rs`, `trade/tests.rs`) sont convertis en
      tests d'erreur 400, et la structure du domain a ses propres tests unitaires couvrant chaque règle ci-dessus.
- [ ] Le front n'appelle plus aucun endpoint paginé en s'appuyant sur un `page_size` par défaut différent de 20.
