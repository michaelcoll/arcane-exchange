# Spec : Filtres de raretés ouvertes à l'échange

## Contexte

`frontend-vue/app/components/Profile/TradeRules.vue` est la dernière section non câblée de l'écran profil. Ses
puces « binders » viennent désormais du backend (spec 019), mais la matrice des raretés reste entièrement
mockée :

- les 4 lignes (M / R / U / C) et leurs compteurs proviennent de `RARITY_DISTRIBUTION` dans
  `frontend-vue/app/utils/trade-rules.ts` ;
- les réglages `on` / `keep` sont un `reactive` local initialisé sur `DEFAULT_RARITY_RULES`, perdu au
  rechargement ;
- la colonne « Proposés », la barre de répartition et `binderFactor` s'appuient sur des poids de binders
  inventés (`BINDER_WEIGHTS`, `FALLBACK_BINDER_WEIGHT`).

La spec 019 laissait explicitement hors scope la distribution par rareté réelle et la persistance des règles.
C'est l'objet de cette spec.

## Objectif

1. Persister, par utilisateur et par rareté, le fait que la rareté est ouverte à l'échange et le nombre
   d'exemplaires toujours gardés.
2. Calculer côté serveur, à partir de la collection réelle et des binders ouverts à l'échange, le nombre
   d'exemplaires possédés et le nombre d'exemplaires réellement proposés pour chaque rareté.
3. Câbler la matrice des raretés de `TradeRules.vue` sur ces données et supprimer les mocks correspondants.

## Solution

### Stockage des règles

- Nouvelle table `collection_rarity_filters` : une ligne par couple (utilisateur, rareté), portant le
  drapeau « ouverte à l'échange » et le nombre d'exemplaires gardés.
- Clé étrangère vers `users(id)` ; la suppression d'un utilisateur emporte ses règles.
- Unicité sur (utilisateur, rareté).
- Le code rareté suit celui déjà utilisé partout (`card.rarity` : `C`, `U`, `R`, `M`, `S`).
- **Par défaut, tout est fermé** : l'absence de ligne vaut « rareté fermée, 0 exemplaire gardé », cohérent
  avec le défaut `private` de la visibilité de collection (spec 017). Aucune ligne n'est créée à
  l'inscription ; les lignes apparaissent au premier réglage.
- Conformément à la règle « une table, un adaptateur » (`.agents/database-schema.instructions.md`), la table
  est portée par un adaptateur dédié.

### Périmètre du calcul

Les compteurs sont calculés **uniquement sur les cartes rangées dans un binder ouvert à l'échange**
(`trading_binders`, spec 019) :

- **Exemplaires** : somme des quantités des entrées de collection de cette rareté dans les binders cochés.
- **Proposés** : pour chaque carte unique de cette rareté dans les binders cochés,
  `max(0, quantité - copies gardées)` ; le total est la somme sur toutes les cartes. Une rareté fermée
  propose 0.

Ne sont listées que les raretés **réellement possédées dans le périmètre** : on ne montre pas à l'utilisateur
une ligne pour une rareté qu'il n'a pas. En conséquence, un utilisateur n'ayant coché aucun binder obtient une
liste vide.

Les raretés sont retournées dans l'ordre d'affichage déjà utilisé côté front (`M`, `R`, `U`, `C`, `S`, cf.
`CollectionFilters.vue`).

### Endpoints

Montés dans le groupe `/collection` (tag `collection`, à côté de `/collection/stats`), authentifiés
(`AuthenticatedUser`, 401 si token absent ou invalide). Ils ne lisent et n'écrivent que les données de
l'utilisateur authentifié.

- `GET /collection/visibility/rarities` : pour chaque rareté possédée dans le périmètre — le code rareté, le
  drapeau ouverte/fermée, le nombre d'exemplaires gardés, le nombre d'exemplaires possédés, le nombre
  d'exemplaires proposés. Calcul à chaque appel, pas de cache.
- `POST /collection/visibility/rarities` : met à jour **une seule rareté** — le corps porte le code rareté, le
  drapeau et le nombre d'exemplaires gardés. Écriture en upsert sur (utilisateur, rareté) : rejouer le même
  appel est sans effet supplémentaire. Réponse sans corps ; le front rappelle le `GET` pour rafraîchir les
  compteurs.

Validation du `POST` :

- code rareté hors des valeurs connues (`C`, `U`, `R`, `M`, `S`) → refus ;
- nombre d'exemplaires gardés hors de l'intervalle `[0, 4]` (`MAX_KEPT_COPIES` côté front) → refus.

### Architecture

Hexagonale, même pattern que `/user/trade-binders` : handler (`adapter_in/collection/controller.rs`) → use
case → service → port repository → adaptateur dédié. Le calcul des exemplaires et des proposés est une
lecture agrégée sur `collection_entry` jointe à `card` (pour la rareté) et restreinte par `trading_binders`.

### Frontend

- `TradeRules.vue` : la matrice est alimentée par `GET /collection/visibility/rarities` ; le toggle et le
  stepper déclenchent `POST` puis un re-`GET` ; cocher ou décocher un binder déclenche également un re-`GET`
  (le périmètre du calcul change).
- Sous chaque rareté, **un seul chiffre** : le nombre d'exemplaires (l'actuel « N cartes · N ex. » perd le
  nombre de cartes uniques).
- La barre de répartition et les totaux (« Proposés », « Gardés par tes règles », « Raretés fermées ») sont
  recalculés à partir des données de l'endpoint, sans plus aucune donnée mockée.
- La ligne rareté `S` doit être affichée comme les autres lorsqu'elle est présente (libellé, pastille,
  couleur d'encre) — les tables de style aujourd'hui limitées à `M`/`R`/`U`/`C` doivent la couvrir.
- Disparition de `RARITY_DISTRIBUTION`, `BINDER_WEIGHTS`, `FALLBACK_BINDER_WEIGHT`, `binderFactor`,
  `eligibleCopies`, `uniqueOf`, `copiesOf`, `TOTAL_COPIES` et `DEFAULT_RARITY_RULES` de `trade-rules.ts`, ainsi
  que des tests unitaires devenus sans objet.
- États de chargement et d'erreur cohérents avec le reste de l'écran (cf. binders, spec 019 : toast d'erreur,
  retour à l'état précédent en cas d'échec d'une écriture).

### Documentation

Documenter les deux endpoints dans `doc/openapi.yml` sous le tag `collection`, à côté de `/collection/stats`.

## Cas d'erreurs

- **Token absent ou invalide** → 401 Unauthorized sur les deux endpoints.
- **Aucun binder ouvert à l'échange** → `GET` retourne 200 avec une liste vide ; l'écran profil affiche un état
  vide explicite invitant à cocher un binder, à la place de la matrice.
- **Collection vide** → même comportement : liste vide, 200.
- **`POST` avec un code rareté inconnu** → 400 Bad Request, aucune écriture en base.
- **`POST` avec un nombre d'exemplaires gardés hors `[0, 4]`** (négatif ou > 4) → 400 Bad Request, aucune
  écriture en base.
- **`POST` sur une rareté que l'utilisateur ne possède pas (ou hors périmètre binders)** → succès : la règle
  est enregistrée, mais la rareté n'apparaît pas dans le `GET` tant qu'elle n'est pas possédée dans le
  périmètre. Ce cas est normal (l'utilisateur peut décocher un binder après avoir réglé une rareté).
- **Erreur base de données** → 500 Internal Server Error (pattern existant).

## Critères d'acceptance

- [ ] Une migration crée la table `collection_rarity_filters` avec une clé étrangère vers `users(id)`, le code
      rareté, le drapeau d'ouverture, le nombre d'exemplaires gardés, et une contrainte d'unicité sur
      (utilisateur, rareté).
- [ ] Given aucune en-tête `Authorization`, When j'appelle `GET` ou `POST /collection/visibility/rarities`,
      Then la réponse est 401 Unauthorized.
- [ ] Given un utilisateur n'ayant coché aucun binder, When j'appelle `GET /collection/visibility/rarities`,
      Then la réponse est 200 avec une liste vide.
- [ ] Given un utilisateur ayant coché « Trade Binder » qui y range 3 exemplaires de cartes rares et 2 de
      communes, et n'ayant jamais rien réglé, When j'appelle `GET`, Then la réponse contient exactement deux
      lignes (`R` et `C`), toutes deux fermées avec 0 exemplaire gardé, 0 proposé, et respectivement 3 et 2
      exemplaires.
- [ ] Given un utilisateur possédant des cartes de rareté `S` dans un binder coché, When j'appelle `GET`, Then
      une ligne `S` est présente ; Given un utilisateur n'en possédant pas, Then aucune ligne `S` n'est
      retournée.
- [ ] Given un utilisateur possédant, dans les binders cochés, une carte rare en 3 exemplaires et une autre en
      1 exemplaire, When la rareté `R` est ouverte avec 1 exemplaire gardé, Then `GET` retourne 4 exemplaires
      et 2 proposés.
- [ ] Given la même situation avec la rareté `R` fermée, When j'appelle `GET`, Then le nombre d'exemplaires
      reste 4 et le nombre de proposés est 0.
- [ ] Given des cartes rares rangées dans un binder non coché, When j'appelle `GET`, Then elles ne sont
      comptées ni dans les exemplaires ni dans les proposés.
- [ ] Given des cartes sans binder (`binder_name` NULL), When j'appelle `GET`, Then elles ne sont jamais
      comptées.
- [ ] Given un utilisateur authentifié, When j'appelle `POST` avec la rareté `M`, ouverte, 2 exemplaires
      gardés, Then la réponse est un succès, une ligne existe en base, et un `GET` reflète le nouveau réglage
      et les proposés recalculés.
- [ ] Given le même appel `POST` rejoué, When la requête est traitée, Then la réponse est un succès et
      `collection_rarity_filters` ne contient toujours qu'une seule ligne pour (utilisateur, `M`).
- [ ] Given un `POST` avec un code rareté inconnu (ex. `X`), When la requête est traitée, Then la réponse est
      400 Bad Request et aucune ligne n'est créée ni modifiée.
- [ ] Given un `POST` avec un nombre d'exemplaires gardés à `-1` ou `5`, When la requête est traitée, Then la
      réponse est 400 Bad Request et aucune ligne n'est créée ni modifiée.
- [ ] Given deux utilisateurs ayant chacun réglé leurs raretés, When l'un appelle `GET`, Then il ne voit que
      ses propres réglages et ses propres compteurs.
- [ ] `TradeRules.vue` affiche une ligne par rareté retournée par l'endpoint, avec le seul nombre
      d'exemplaires sous le libellé (plus de compteur de cartes uniques).
- [ ] Given un changement de toggle ou de stepper sur l'écran profil, When l'appel réussit, Then les colonnes
      « Proposés » et la barre de répartition reflètent les valeurs recalculées par le serveur, et le réglage
      survit à un rechargement de la page.
- [ ] Given un échec de l'appel `POST`, When l'erreur remonte, Then l'affichage revient à l'état précédent et
      un toast d'erreur est affiché.
- [ ] Given l'utilisateur coche ou décoche un binder, When l'appel aboutit, Then la matrice des raretés est
      rechargée et ses compteurs changent en conséquence.
- [ ] Given un utilisateur sans binder coché, When il ouvre l'écran profil, Then un état vide explicite est
      affiché à la place de la matrice, sans erreur en console.
- [ ] `RARITY_DISTRIBUTION`, `BINDER_WEIGHTS`, `FALLBACK_BINDER_WEIGHT` et `binderFactor` n'existent plus dans
      `trade-rules.ts`.
- [ ] Les deux endpoints sont documentés dans `doc/openapi.yml`.
- [ ] `mise run checks` passe sans erreur (inclut `rebuild-db-doc`, `sqlx-prepare`, tests et lint).
- [ ] `mise run lint-frontend` passe sans erreur.

## Hors scope

- Application effective des règles de rareté dans les endpoints existants (recherche, offres, visibilité
  `trade`, consultation de la collection d'un tiers) — cette spec stocke, calcule et expose, elle ne filtre
  nulle part ailleurs.
- Modification en masse (régler plusieurs raretés en un appel) : le `POST` traite une rareté à la fois.
- Règles plus fines que « par rareté » (par set, par carte, par prix).
- Purge des lignes `collection_rarity_filters` à l'import ManaBox : les réglages par rareté restent valides
  quel que soit le contenu de la collection.
