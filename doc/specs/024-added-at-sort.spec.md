# Spec : Tri par date d'ajout (`added_at`)

## Contexte

`GET /collection` et `GET /search/card` partagent aujourd'hui le même mécanisme de tri, porté par les enums du
domain `CollectionSortField` (`avg`, `trend`, `set_code`, `language_code`) et `SortDirection` (`asc`, `desc`), exposés
côté HTTP par `SortByParam`/`SortDirParam` (`collection/dto.rs`, réutilisés par `search/dto.rs`, cf.
`008-search-endpoint.spec.md`). Le tri est interpolé directement dans la clause `ORDER BY` de la requête SQL
(`card_prices_view_repository_adapter.rs`).

En pratique, seul le prix (`trend`) est exposé à l'utilisateur : la page collection affiche un bouton qui inverse
`sort_dir` (asc/desc) mais aucun moyen de changer `sort_by` (toujours `trend`) ; la page recherche ne propose aucun
contrôle de tri du tout, bien que `sort_by`/`sort_dir` existent dans son état frontend (fixés à `trend`/`desc`,
jamais exposés).

La colonne `added_at` existe déjà (`collection_entry`, vue matérialisée `mv_card_prices`) et est déjà remontée dans
la réponse de `GET /collection`. Sur `GET /search/card`, elle est aujourd'hui systématiquement forcée à `NULL`,
quel que soit le filtre appliqué : chaque ligne de résultat peut regrouper plusieurs propriétaires (`GROUP BY` +
`owner_count`, cf. `009-search-owner-count.spec.md`), donc il n'existe pas de valeur `added_at` unique par ligne dans
le cas général. Ce n'est que lorsque `player_username` restreint la recherche à un propriétaire unique que la donnée
redevient bien définie individuellement — un cas déjà traité pour la colonne `reserved`, dont le calcul réel n'est
déclenché que dans ce même contexte.

## Objectif

Permettre de trier par date d'ajout (`added_at`), avec inversion du sens (asc/desc), partout où l'ordre qu'elle
induit a un sens :

- `GET /collection` (toujours scopée à l'utilisateur authentifié) : support complet. La page collection s'ouvre par
  défaut triée par date d'ajout.
- `GET /search/card` restreinte à un joueur (`player_username` fourni, propriétaire unique garanti) : le tri par
  `added_at` devient possible et actif par défaut dès qu'un joueur est ciblé. La valeur `added_at` elle-même n'est
  **pas** exposée dans la réponse (périmètre volontairement limité au tri ; elle n'est affichée nulle part dans l'UI
  aujourd'hui, y compris sur la collection perso où elle est pourtant déjà disponible).
- `GET /search/card` non restreinte (recherche générale multi-propriétaires) : le tri par `added_at` n'est pas
  supporté, la donnée restant ambiguë par ligne ; le tri par défaut reste le prix, inchangé.

Côté UI, le bouton unique d'inversion du prix (collection) est remplacé par deux contrôles de tri indépendants
(prix, date d'ajout), chacun sélectionnable et inversable, sur le même modèle visuel. Le même duo de contrôles est
introduit sur la page recherche (qui n'en a aucun aujourd'hui), avec le contrôle "date d'ajout" actif uniquement
quand un joueur est effectivement ciblé.

## Solution

### Domaine

- `CollectionSortField` gagne une variante `AddedAt`, utilisable comme n'importe quelle autre valeur de tri par le
  mécanisme existant.
- Aucun changement de la valeur par défaut du domaine (`CollectionSortField` reste `Trend` par défaut, `SortDirection`
  reste `Desc`) : `/search/card` appelé sans aucun paramètre doit continuer à se comporter exactement comme
  aujourd'hui (voir Validation ci-dessous — un défaut à `AddedAt` romprait ce cas, `GET /search/card` étant utilisé
  aussi bien scopé que non scopé). Le tri "par défaut" décrit dans l'Objectif pour la collection et pour la
  recherche scopée est une valeur initiale poussée explicitement par le frontend, pas un nouveau défaut serveur.

### API / DTO

- `SortByParam` (`collection/dto.rs`, réutilisé par `search/dto.rs`) gagne la valeur `added_at`, sans changement de
  défaut (cf. ci-dessus).
- Aucun nouveau paramètre de requête : le mécanisme `sort_by`/`sort_dir` existant suffit.
- Le schéma de réponse de `GET /search/card` reste inchangé : aucun champ `added_at` n'est ajouté à la réponse, que
  `player_username` soit fourni ou non, et quel que soit le `sort_by` demandé. Seul l'**ordre** des résultats est
  affecté.
- Les bindings TypeScript générés (`SortBy.ts`) sont régénérés pour inclure `'added_at'`.

### Repository — tri par `added_at` en recherche scopée

- Le tri par `added_at` n'est activé, au niveau requête, que lorsque la recherche est restreinte à un propriétaire
  unique (`player_username` fourni). Dans le cas général (non scopé), il n'est pas utilisable (cf. Validation).
- Quand plusieurs entrées d'un même joueur correspondent à une même carte regroupée (ex. plusieurs exemplaires
  répartis sur différents binders), la date retenue pour le tri est la plus ancienne des dates d'ajout
  correspondantes, et non la plus récente : cette agrégation (`MIN`) existe déjà en amont, dans la vue partagée
  utilisée aussi bien par cette fonctionnalité que par la collection personnelle — elle est hors périmètre de cette
  spec et n'est pas modifiée ici. Le tri par défaut "plus récent d'abord" (cf. Objectif) reste correct pour le cas
  courant (une seule entrée par carte) ; seul le cas d'exemplaires multiples pour une même carte s'écarte de cette
  intuition. Cette valeur ne devient pas une colonne de la réponse (cf. API/DTO ci-dessus).
- Quand un filtre texte (`q`) est actif, le classement par pertinence textuelle reste prioritaire sur le tri
  choisi, comme c'est déjà le cas aujourd'hui pour le tri par prix : ce n'est pas une régression introduite par
  cette fonctionnalité, le comportement de précédence est inchangé et s'applique identiquement à `added_at`.

### Validation — tri par `added_at` non disponible en recherche non scopée

- Une requête `GET /search/card?sort_by=added_at` sans `player_username` effectif (paramètre absent ou vide — une
  chaîne vide étant déjà traitée comme absente, cf. `013-search-player-username-filter.spec.md`) est rejetée avant
  toute exécution de requête, avec une nouvelle erreur fonctionnelle dédiée, mappée en `400 Bad Request` au même
  format que les erreurs existantes.
- Cette règle est portée par le domain (au même niveau que les règles de pagination, cf.
  `022-pagination-domain-rules.spec.md`), puisque `SearchQuery` porte déjà à la fois `sort_by` et
  `player_username` : elle est vérifiée à la construction de la requête de recherche, avant d'atteindre le
  repository, et invoquée depuis le contrôleur comme le sont les règles de pagination.
- `GET /collection` n'est jamais concerné par cette validation (toujours scopée à l'utilisateur authentifié, jamais
  de notion de `player_username`).

### Frontend — collection

- `pages/collection/index.vue` : le bouton actuel d'inversion du tri prix est remplacé par deux contrôles de tri
  indépendants (prix, date d'ajout), sur le même modèle visuel que l'existant. Sélectionner un contrôle fixe
  `sort_by` sur le champ correspondant et bascule son propre `sort_dir` ; les deux contrôles sont mutuellement
  exclusifs (un seul `sort_by` actif à la fois, cohérent avec le domain).
- État initial de la page : la première requête est envoyée avec `sort_by=added_at`, `sort_dir=desc` explicites (pas
  de dépendance à un défaut serveur, cf. Domaine ci-dessus).
- Un changement de tri réinitialise la pagination et la liste accumulée, comme c'est déjà le cas aujourd'hui.

### Frontend — recherche

- `pages/search/index.vue` : ajout des deux mêmes contrôles de tri (aucun n'existe aujourd'hui ; `sort_by`/
  `sort_dir` sont déjà dans l'état du composant mais inutilisés). Un changement de tri réinitialise la pagination et
  la liste accumulée, sur le même principe que la page collection.
- Tant qu'aucun joueur n'est **effectivement** ciblé (mode recherche générale, ou mode "joueur" sans joueur encore
  résolu — y compris pendant la résolution asynchrone d'un lien profond du type `/search?player=alice`), seul le
  contrôle "prix" est visible/actif (désormais inversable, ce qui est nouveau sur cet écran) ; le contrôle "date
  d'ajout" est masqué. `sort_by` reste sur le prix dans cet état, pour ne jamais émettre `sort_by=added_at` sans
  `player_username` effectif.
- Dès que `player_username` est effectivement posé (joueur résolu), le contrôle "date d'ajout" devient disponible et
  le tri repasse automatiquement sur `added_at`/`desc` par défaut, avec rafraîchissement des résultats.
- Dès que le ciblage joueur est retiré, par quelque chemin que ce soit (effacement explicite du joueur, retour en
  mode recherche générale), le tri retombe automatiquement sur le prix par défaut (`sort_by` et `sort_dir` réinitialisés
  ensemble, pas seulement `sort_by`), avant que la requête suivante ne parte — la combinaison
  `sort_by=added_at` sans `player_username` n'est jamais émise par le frontend.

## Cas d'erreurs

- `GET /search/card?sort_by=added_at` sans `player_username` (absent) → `400`, message précisant que ce tri
  nécessite un `player_username`.
- `GET /search/card?sort_by=added_at&player_username=` (vide) → traité comme absent (cf. spec 013), donc `400` au
  même titre que ci-dessus.
- `GET /search/card?sort_by=added_at&player_username=<joueur inconnu>` → comportement inchangé par rapport à
  `013-search-player-username-filter.spec.md` : `200`, liste vide, `total = 0` (la présence non vide de
  `player_username` suffit à passer la validation ci-dessus, son existence n'est pas vérifiée à ce niveau).
- `GET /search/card` sans `player_username` ni `sort_by` → comportement inchangé (tri par défaut prix).
- Les autres valeurs de `sort_by` (`avg`, `trend`, `set_code`, `language_code`) restent inchangées sur les deux
  endpoints, y compris en recherche non scopée.

## Critères d'acceptance

- [ ] `GET /collection?sort_by=added_at&sort_dir=asc` retourne les cartes du propriétaire connecté triées par
      `added_at` croissant (plus ancien d'abord).
- [ ] `GET /collection?sort_by=added_at&sort_dir=desc` retourne les cartes triées par `added_at` décroissant (plus
      récent d'abord).
- [ ] `GET /collection` sans paramètre de tri conserve son comportement actuel (tri par défaut prix, `trend`/`desc`,
      inchangé côté serveur).
- [ ] Given des cartes ajoutées à des dates connues à la collection d'un joueur, when
      `GET /search/card?player_username=<ce joueur>&sort_by=added_at&sort_dir=asc|desc` est appelé, then les cartes
      apparaissent dans l'ordre chronologique attendu (croissant ou décroissant selon `sort_dir`).
- [ ] Le schéma de réponse de `GET /search/card` est inchangé par cette fonctionnalité : aucun champ `added_at`
      n'apparaît dans la réponse, avec ou sans `player_username`, quel que soit `sort_by`.
- [ ] `GET /search/card?sort_by=added_at` sans `player_username` (absent ou vide) → `400`, message indiquant que ce
      tri nécessite un `player_username`.
- [ ] `GET /search/card?player_username=<joueur inconnu>&sort_by=added_at` → `200`, liste vide, `total = 0` (pas une
      erreur de validation).
- [ ] Given un filtre texte `q` actif sur `/search/card`, when `sort_by=added_at` est demandé avec `player_username`,
      then le classement par pertinence textuelle reste prioritaire sur le tri par date, comme c'est déjà le cas
      aujourd'hui pour le tri par prix.
- [ ] Les tris existants (`avg`, `trend`, `set_code`, `language_code`) continuent de fonctionner à l'identique sur
      `/collection` et `/search/card`, scopé ou non.
- [ ] La page collection affiche deux contrôles de tri indépendants (prix, date d'ajout), chacun inversable
      (asc/desc), mutuellement exclusifs.
- [ ] La page collection charge par défaut avec le tri "date d'ajout" actif, en décroissant (la requête initiale
      envoie explicitement `sort_by=added_at&sort_dir=desc`).
- [ ] La page recherche, en mode général (aucun joueur ciblé) ou en mode "joueur" avant résolution du joueur,
      n'affiche que le contrôle de tri "prix" (désormais inversable) ; le contrôle "date d'ajout" est masqué et
      `sort_by` reste sur le prix.
- [ ] La page recherche, dès qu'un joueur est effectivement résolu (y compris via un lien profond
      `/search?player=<username>`), affiche les deux contrôles et repasse automatiquement sur le tri "date d'ajout"
      décroissant, avec rafraîchissement des résultats.
- [ ] La page recherche, dès que le ciblage joueur est retiré (effacement explicite ou retour en mode général) alors
      que le tri actif est "date d'ajout", réinitialise `sort_by` et `sort_dir` sur le prix par défaut avant toute
      requête suivante ; aucune requête `sort_by=added_at` sans `player_username` n'est jamais émise.
- [ ] Un changement de tri, sur la page collection comme sur la page recherche, réinitialise la pagination et la
      liste de résultats accumulée.
- [ ] `doc/openapi.yml` inclut `added_at` dans l'énumération `SortBy` (généré automatiquement) et documente
      explicitement le cas `400` sur `/search/card` dans l'annotation de l'endpoint.
- [ ] Le binding `SortBy.ts` régénéré inclut `'added_at'`.
- [ ] Les tests backend couvrent : tri collection par `added_at` asc/desc, tri recherche scopée par `added_at`
      asc/desc (vérifié par l'ordre des cartes retournées, sans dépendre d'un champ `added_at` en réponse), rejet
      `400` du tri `added_at` sans `player_username` (absent et vide), `player_username` inconnu + tri `added_at` →
      liste vide (pas une erreur), absence du champ `added_at` dans le schéma de réponse de `/search/card`.
- [ ] `mise run lint-backend`, `mise run lint-frontend` et `mise run format` passent sans erreur.
