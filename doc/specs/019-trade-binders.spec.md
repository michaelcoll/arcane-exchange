# Spec : Binders ouverts à l'échange

## Contexte

La spec 018 a introduit `collection_entry.binder_name` : chaque exemplaire de carte importé depuis ManaBox
connaît désormais son binder d'origine. Cette donnée n'est aujourd'hui exposée nulle part (ni API, ni
frontend) — elle sert uniquement, en interne, à préserver l'agrégation des lectures existantes.

Côté profil, `frontend-vue/app/components/Profile/TradeRules.vue` affiche déjà une rangée de puces
« Périmètre · binders ManaBox », mais :

- la liste des binders vient de la constante mockée `BINDERS` (`frontend-vue/app/utils/trade-rules.ts`),
  dont le commentaire annonce explicitement l'arrivée de vraies données ;
- la sélection est un simple `ref(['trade', 'bulk'])`, non persistée — elle est perdue au rechargement.

`GET /collection/stats` retourne aujourd'hui `total_cards`, `unique_cards`, `price_trend_min`,
`price_trend_max` et `sets`. C'est déjà le point d'entrée des données agrégées de collection, et l'écran
profil n'a pas d'endpoint de stats dédié.

## Objectif

1. Exposer la liste réelle des binders de l'utilisateur, avec le nombre d'exemplaires rangés dans chacun,
   afin d'alimenter les puces de l'écran profil.
2. Permettre à l'utilisateur de choisir quels binders sont ouverts à l'échange, et persister ce choix.
3. Câbler les puces binders de `TradeRules.vue` sur ces données.

## Solution

### Liste des binders dans `/collection/stats`

- Ajouter au retour de `GET /collection/stats` une liste `binders`, chaque élément portant le nom du
  binder et son nombre d'exemplaires (somme des quantités des entrées de collection de ce binder pour
  l'utilisateur courant).
- Les entrées dont `binder_name` est `NULL` (cartes sans binder) sont **exclues** de la liste et ne
  forment pas d'entrée « Non classé ».
- Tri par nombre d'exemplaires décroissant.
- Endpoint inchangé par ailleurs : même route, même authentification, calcul à chaque appel, pas de cache.

### Stockage de la sélection

- Nouvelle table `trading_binders` : une ligne par binder sélectionné, avec une clé étrangère vers
  `users(id)` et une colonne portant le nom du binder.
- Unicité sur le couple (utilisateur, nom de binder) : un même binder ne peut être sélectionné deux fois.
- Aucune sélection par défaut : un utilisateur qui n'a jamais rien coché a une liste vide.
- La suppression d'un utilisateur emporte ses sélections.
- Conformément à la règle « une table, un adaptateur » (`.agents/database-schema.instructions.md`), la
  table est portée par un adaptateur dédié.

### Endpoints

Montés sous le même groupe que `/user/register` et `/user/visibility` (tag `auth`), authentifiés
(`AuthenticatedUser`, 401 si token absent ou invalide). Ils ne lisent et n'écrivent que la sélection de
l'utilisateur authentifié.

- `GET /user/trade-binders` : liste des noms de binders sélectionnés.
- `POST /user/trade-binders` : ajoute un binder à la sélection, son nom étant porté par le corps de la
  requête. Opération idempotente : ajouter un binder déjà sélectionné n'est pas une erreur et ne crée pas
  de doublon.
- `DELETE /user/trade-binders/{name}` : retire un binder de la sélection, son nom étant porté par le path
  et donc URL-encodé (les noms ManaBox contiennent espaces, accents et ponctuation). Opération idempotente
  également.

`POST` valide que le nom correspond à un binder réellement présent dans la collection de l'utilisateur
(`collection_entry.binder_name` non nul) : on ne sélectionne pas un binder qui n'existe pas.

Ces endpoints exposent uniquement les noms des binders sélectionnés — le nombre d'exemplaires reste porté
par `/collection/stats`, seule source de comptage.

### Purge à l'import

L'import ManaBox remplace intégralement les entrées de collection de l'utilisateur (`delete_all` puis
réinsertion). Un binder renommé, vidé ou disparu du nouveau CSV laisserait donc une sélection orpheline.
À l'issue de l'import, les lignes de `trading_binders` dont le nom ne correspond plus à aucun binder de la
collection de l'utilisateur sont supprimées. Les autres sélections sont conservées telles quelles.

### Architecture

Hexagonale, même pattern que `/user/visibility` : handler (`adapter_in/user/controller.rs`) → use case →
service → port repository → adaptateur. L'ajout de `binders` dans les stats étend la chaîne existante
`collection_stats` (domaine, service, `collection_stats_repository_adapter`) sans en changer la forme.

### Frontend

- `TradeRules.vue` : les puces binders sont alimentées par le champ `binders` de `/collection/stats`
  (nom et nombre d'exemplaires affichés) ; la sélection initiale vient de `GET /user/trade-binders` ;
  cocher une puce déclenche `POST`, la décocher `DELETE`.
- La constante mockée `BINDERS` disparaît de `trade-rules.ts`.
- **Contrainte** : le calcul « Proposés » reste, dans cette spec, adossé aux données mockées
  (`RARITY_DISTRIBUTION` et les poids par rareté et par binder utilisés par `binderFactor`), le backend ne
  fournissant pas encore la distribution par rareté. Les binders réels n'ayant pas les clés du mock, il
  faut un poids de repli pour tout binder non couvert par la table de poids mockée, sans faire tomber
  l'affichage à zéro ni provoquer d'erreur.
- États de chargement et d'erreur cohérents avec les autres écrans déjà câblés sur l'API (cf. sélecteur de
  visibilité de la spec 017, sur le même écran).

### Documentation

Documenter les trois endpoints dans `doc/openapi.yml` sous le tag `auth`, à côté de `/user/visibility`, et
mettre à jour le schéma de réponse de `/collection/stats`.

## Cas d'erreurs

- **Token absent ou invalide** → 401 Unauthorized sur les trois endpoints (comportement standard).
- **Utilisateur authentifié jamais enregistré dans `users`** → 404 Not Found, cohérent avec
  `/user/visibility` (la clé étrangère de `trading_binders` interdit d'ailleurs l'insertion).
- **`POST` avec un nom de binder absent de la collection de l'utilisateur** → refus explicite (4xx),
  aucune écriture en base.
- **`POST` avec un nom vide ou composé uniquement d'espaces** → refus explicite (4xx) : ce n'est jamais un
  binder valide, la spec 018 normalisant ces valeurs à `NULL` à l'import.
- **`POST` d'un binder déjà sélectionné** → succès, sans doublon en base.
- **`DELETE` d'un binder non sélectionné (ou inexistant)** → succès, sans effet.
- **Collection vide ou sans aucun binder nommé** → `binders` est une liste vide dans `/collection/stats`
  (pas d'erreur), et l'écran profil affiche un état vide explicite plutôt qu'une rangée de puces vide.
- **Erreur base de données** → 500 Internal Server Error (pattern existant).

## Critères d'acceptance

- [ ] Une migration crée la table `trading_binders` avec une clé étrangère vers `users(id)`, le nom du
      binder, et une contrainte d'unicité sur (utilisateur, nom).
- [ ] Given un utilisateur possédant 4 exemplaires dans « Trade Binder » et 2 dans « Bulk », When
      j'appelle `GET /collection/stats`, Then `binders` contient exactement ces deux binders, avec 4 et 2
      exemplaires, « Trade Binder » en premier.
- [ ] Given un utilisateur dont certaines cartes ont `binder_name = NULL`, When j'appelle
      `GET /collection/stats`, Then aucune entrée ne correspond à ces cartes dans `binders`, et
      `total_cards` / `unique_cards` restent inchangés.
- [ ] Given un utilisateur avec une collection vide, When j'appelle `GET /collection/stats`, Then
      `binders` est une liste vide et l'endpoint retourne 200.
- [ ] Given aucune en-tête `Authorization`, When j'appelle `GET`, `POST` ou `DELETE` sur
      `/user/trade-binders`, Then la réponse est 401 Unauthorized.
- [ ] Given un utilisateur authentifié sans sélection, When j'appelle `GET /user/trade-binders`, Then la
      réponse est 200 avec une liste vide.
- [ ] Given un utilisateur possédant le binder « Trade Binder », When j'appelle `POST /user/trade-binders`
      avec ce nom, Then la réponse est un succès et une ligne est créée dans `trading_binders`, et
      `GET /user/trade-binders` la retourne.
- [ ] Given le même contexte, When j'appelle `POST` une seconde fois avec le même nom, Then la réponse est
      un succès et `trading_binders` ne contient toujours qu'une seule ligne pour ce binder.
- [ ] Given un utilisateur ne possédant pas le binder « Inconnu », When j'appelle `POST` avec ce nom, Then
      la réponse est une erreur 4xx et aucune ligne n'est créée.
- [ ] Given un `POST` avec un nom vide ou composé d'espaces, When la requête est traitée, Then la réponse
      est une erreur 4xx et aucune ligne n'est créée.
- [ ] Given un binder sélectionné dont le nom contient un espace et un accent (ex : « Binder à échanger »),
      When j'appelle `DELETE /user/trade-binders/{name}` avec le nom URL-encodé, Then la sélection est
      supprimée et `GET /user/trade-binders` ne la retourne plus.
- [ ] Given un binder non sélectionné, When j'appelle `DELETE` avec son nom, Then la réponse est un succès
      et la sélection des autres binders est inchangée.
- [ ] Given deux utilisateurs ayant chacun sélectionné des binders, When l'un appelle
      `GET /user/trade-binders`, Then il ne voit que ses propres sélections.
- [ ] Given un utilisateur ayant sélectionné « Bulk » et « Decks », When il ré-importe un CSV où « Decks »
      n'apparaît plus, Then `trading_binders` ne contient plus que « Bulk » après l'import.
- [ ] Given un utilisateur ayant sélectionné « Bulk », When il ré-importe un CSV contenant toujours
      « Bulk », Then la sélection est conservée.
- [ ] `TradeRules.vue` affiche les binders réels de l'utilisateur avec leur nombre d'exemplaires, coche
      ceux déjà sélectionnés au chargement, et persiste tout changement (vérifié par rechargement de la
      page).
- [ ] Given un utilisateur sans aucun binder nommé, When il ouvre l'écran profil, Then un état vide
      explicite est affiché à la place des puces, sans erreur en console.
- [ ] La constante `BINDERS` n'est plus utilisée par `TradeRules.vue`.
- [ ] Les trois endpoints et le nouveau champ `binders` sont documentés dans `doc/openapi.yml`.
- [ ] `mise run checks` passe sans erreur (inclut `rebuild-db-doc`, `sqlx-prepare`, tests et lint).
- [ ] `mise run lint-frontend` passe sans erreur.

## Hors scope

- Filtrage effectif des cartes proposées à l'échange selon les binders sélectionnés (recherche, offres,
  visibilité `trade`) — cette spec stocke et expose la sélection, elle ne l'applique nulle part.
- Distribution par rareté réelle et calcul serveur des « Proposés » : `RARITY_DISTRIBUTION` et les poids
  par binder restent mockés côté front.
- Persistance des règles de rareté (`on` / `keep`) de `TradeRules.vue` — toujours un état local.
- CRUD des binders eux-mêmes (création, renommage) : ManaBox reste la source de vérité, `binder_name` est
  une donnée importée en lecture seule.
- Exposition de `binder_name` dans la liste de collection ou la recherche.
