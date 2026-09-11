# Spec : Sortir le foil de l'identité de la carte

## Contexte

La table `card` est le catalogue des **définitions** de cartes : nom, rareté, `scryfall_id`, `cardmarket_id`,
`the_gatherer_id`. Or `foil` fait partie de sa clé primaire
(`(set_code, collector_number, language_code, foil)`, migration `0001_init_database.sql`), et cette « identité de
carte » est documentée comme un invariant du schéma (`.agents/database-schema.instructions.md`).

C'est une erreur de modélisation : le foil n'est pas une propriété de la définition d'une carte, c'est une propriété
de l'**exemplaire** qu'un joueur possède ou échange. Une même impression — même set, même numéro de collection, même
langue — existe en version normale et en version foil, et les deux partagent le même `scryfall_id`, le même
`cardmarket_id` et le même `the_gatherer_id` (la spec `004` le documente déjà : « un `scryfall_id` par printing y
compris foil/non-foil »).

Conséquences observables aujourd'hui :

- Le catalogue contient jusqu'à **deux lignes strictement redondantes** par impression, ne différant que par un
  booléen — toutes les autres colonnes étant identiques par construction. Mesure sur la base de développement :
  241 clés `(set_code, collector_number, language_code)` portent deux variantes, pour 6 991 lignes de catalogue.
- `find_by_scryfall_id` (`card_repository_adapter.rs:65`) fait un `fetch_optional` sur
  `WHERE scryfall_id = $1` alors que plusieurs lignes peuvent correspondre (variantes foil, et variantes de langue —
  289 `scryfall_id` ont plusieurs lignes, dont 50 s'étendent sur plusieurs langues). La ligne retournée est
  **arbitraire**, et c'est elle qui décide quelles colonnes de prix (`low` ou `low_foil`) alimentent l'historique de
  prix par carte. L'endpoint peut donc déjà renvoyer la mauvaise série, sans erreur ni signal.
- Chaque flux qui enrichit le catalogue (résolution Cardmarket, mise à jour Gatherer) traite deux fois la même carte,
  ces flux filtrant explicitement sur `foil` (`card_repository_adapter.rs:159` et `:180`).

Les tables `collection_entry` et `trade_card` portent déjà une colonne `foil`, mais uniquement parce qu'elle est
**partie de leur clé étrangère** vers `card`. La donnée est au bon endroit ; c'est le sens qu'il faut corriger.

## Objectif

Faire de `card` un catalogue de définitions pures, sans notion de finition : une ligne par
`(set_code, collector_number, language_code)`. Le foil devient un attribut autonome de la possession
(`collection_entry`) et de l'échange (`trade_card`), porté par l'exemplaire et non par la définition.

Le comportement fonctionnel visible par l'utilisateur reste inchangé sur la collection, la recherche et les
échanges : une carte foil et sa version normale restent deux entrées distinctes, avec leurs prix respectifs. Ce qui
change, c'est la source de l'information foil dans toute la chaîne — et deux points de contrat détaillés plus bas
(l'historique de prix par carte, et le compteur de cartes du catalogue).

## Solution

### Modèle de données

- La clé primaire de `card` devient `(set_code, collector_number, language_code)` et la colonne `foil` est retirée
  de la table.
- `collection_entry.foil` et `trade_card.foil` sont **conservées telles quelles** : mêmes colonnes, mêmes types,
  toujours `NOT NULL` avec `false` comme valeur par défaut fonctionnelle. Elles cessent d'être une partie de la clé
  étrangère vers `card` et deviennent des attributs propres.
- Les clés d'unicité de ces deux tables continuent d'inclure `foil` : il reste légitime de posséder à la fois la
  version normale et la version foil de la même carte, dans le même classeur, et de les engager séparément dans un
  échange. Seules les **clés étrangères** vers `card` perdent la colonne.
- La migration est forward-only, conformément aux conventions du projet.

### Migration des données existantes

- Les lignes de `card` sont dédupliquées sur `(set_code, collector_number, language_code)` : une seule survit par
  clé, portant pour chaque colonne la valeur non nulle disponible parmi les variantes fusionnées.
- Cette fusion repose sur l'hypothèse que `name`, `rarity`, `scryfall_id`, `cardmarket_id` et `the_gatherer_id` sont
  identiques entre les deux variantes — deux cartes de même set et même numéro de collection désignant la même
  impression. L'hypothèse doit être **vérifiée par la migration elle-même**, pas supposée.
- La vérification doit distinguer deux situations, faute de quoi elle transformerait un état sain en panne :
  - **une valeur d'un côté, `NULL` de l'autre** sur `cardmarket_id` ou `the_gatherer_id` : état **normal et
    attendu**. Ces colonnes sont renseignées après coup, par des flux asynchrones qui traitent chaque variante foil
    indépendamment (`update_cardmarket_id` et `update_gatherer_id` filtrent sur `foil`). Une variante peut donc être
    résolue avant l'autre. La fusion retient la valeur non nulle.
  - **deux valeurs non nulles divergentes** sur l'une de ces colonnes, ou une divergence sur `name`, `rarity` ou
    `scryfall_id` : véritable anomalie, la migration s'arrête. Voir Cas d'erreurs.
- Aucune donnée de `collection_entry` ni de `trade_card` n'est modifiée : leur colonne `foil` porte déjà la bonne
  valeur pour chaque exemplaire. La migration se limite au catalogue et à la reprise des contraintes.

### Domaine

- L'identité de carte du domaine (`CardId`, `domain/card.rs`) perd son champ `foil`, ainsi que sa représentation
  textuelle (le symbole `⭑`, qui appartient désormais à l'exemplaire).
- Le foil est porté au niveau de l'entrée de collection et de la carte d'échange, aux côtés de l'identité de carte.
  Il ne doit plus être possible de construire une identité de carte foil.
- Les entités de persistance et leurs conversions suivent ce découpage.

### Import de collection

- Le champ `Foil` du CSV (colonne 7, `parse_service.rs:162`) continue d'être lu avec la même règle : toute valeur
  autre que `normal` vaut foil. Ce qui change est sa destination : l'entrée de collection, plus la définition de
  carte.
- La déduplication doit être **dissociée** : sur l'identité de carte seule côté catalogue, sur l'identité de carte
  **plus la finition** côté collection. Deux points du flux d'import sont concernés, et le premier est le plus
  dangereux :
  - `parse_service.rs:73` déduplique les lignes du CSV sur la clé `(CardId, binder_name)` et **fusionne les entrées
    en sommant les quantités**. Dès que `CardId` perd le foil, une ligne `normal` et une ligne `foil` du même
    classeur deviennent la même clé : elles seraient silencieusement fusionnées en une seule entrée de quantité 2,
    et la version foil disparaîtrait de la collection. Cette clé doit donc explicitement porter la finition.
  - `card_repository_adapter.rs:88` insère le catalogue par `INSERT ... ON CONFLICT DO UPDATE`, qui ne peut pas
    affecter deux fois la même ligne dans un seul ordre. La déduplication qui alimente cet ordre doit, elle, se
    faire sur l'identité de carte **sans** la finition.
- Un CSV sans information de finition exploitable produit une entrée non-foil ; il ne doit jamais faire échouer la
  ligne.

### Résolution de la finition dans les lectures de prix

Le prix Cardmarket dépend de la finition, et cette dépendance doit être préservée partout. **Six** emplacements la
tirent aujourd'hui de `card.foil` et doivent la tirer de l'exemplaire — `collection_entry.foil` ou
`trade_card.foil` selon le contexte. Aucun ne doit être oublié : la suppression de `card.foil` ne provoque une
erreur de compilation que sur les requêtes vérifiées par `sqlx`, mais une finition mal réorientée produirait des
prix faux, silencieusement.

- `mv_last_cardmarket_prices` conserve sa clé `(set_code, collector_number, foil)` et son index unique, mais ne peut
  plus dériver la finition du catalogue : elle expose désormais **les deux finitions pour chaque carte du
  catalogue**, avec les prix correspondants. La vue cesse ainsi de refléter ce que les utilisateurs possèdent pour
  refléter ce qui est cotable — ce qui est cohérent avec l'objectif de la spec `028` (un prix accessible
  indépendamment de la possession). **Cette spec amende le critère d'acceptance de la `028`** qui exigeait
  « exactement une ligne par `(set_code, collector_number, foil)` distinct de la table `card` ».
- `mv_card_prices` joint `collection_entry` au catalogue sur trois colonnes, et prend sa finition dans
  `collection_entry.foil` pour la jointure aux prix. Sa clé unique
  `(set_code, collector_number, language_code, foil, user_id)` et son contenu restent inchangés.
- `v_tradable_entry` prend également sa finition dans `collection_entry`, son grain reste identique.
- `collection_price_history_repository_adapter.rs:53` calcule la valeur quotidienne de la collection d'un
  utilisateur en sélectionnant les colonnes de prix selon `c.foil`, et **écrit le résultat en base**
  (`collection_price_history`). C'est le point le plus sensible de cette liste : une finition mal réorientée y
  produirait un historique de valeur faux et **persisté**, invisible jusqu'à ce qu'un utilisateur remarque une
  rupture dans sa courbe. La finition doit venir de `collection_entry`.
- `find_trade_cards_with_details` (`trade_repository_adapter.rs:176`) joint `card` puis
  `mv_last_cardmarket_prices` en faisant transiter la finition par `card`. Elle doit la prendre directement dans
  `trade_card.foil`, ce qui supprime au passage un maillon de la jointure.
- `collection_stats_repository_adapter.rs:64` et `collection_rarity_filters_repository_adapter.rs:41` joignent
  `card` à `collection_entry` sur quatre colonnes pour y lire la rareté. Ces jointures passent à trois colonnes ;
  elles ne concernent pas les prix, mais une jointure laissée à quatre colonnes ne compilerait plus.

Les contraintes de rafraîchissement posées par la spec `028` restent en vigueur, ordre compris : la vue des derniers
prix se rafraîchit avant `mv_card_prices`. Une vue matérialisée ne se redéfinit pas en place : elle est supprimée et
recréée dans la même migration, index compris.

### API

Le foil est déjà exposé au bon niveau dans les charges utiles : à côté de l'identité de carte, jamais imbriqué dans
une définition. La surface publique change donc peu.

- Inchangés, en requête comme en réponse : `CollectionCardResponse`, `TradeCardResponse`, `AddTradeCardRequest`,
  `RemoveTradeCardRequest`, `CardOffersParams`.
- `GET /card/{scryfall_id}/price-history` gagne un paramètre de requête `foil` **obligatoire**, sur le modèle de
  celui de `GET /card/offers`. La carte ne portant plus la finition, l'endpoint ne peut plus la deviner — et cesse
  du même coup de la deviner mal. C'est la seule rupture de contrat de cette spec : les appelants doivent être mis à
  jour.
- **Contrainte impérative** : cet endpoint partage aujourd'hui son DTO de paramètres de requête
  (`PriceHistoryParams`) avec `GET /collection/price-history`, qui n'a aucune notion de finition. Le paramètre `foil`
  ne doit pas être ajouté au DTO partagé — cela rendrait `/collection/price-history` systématiquement rejeté en 400.
  Les deux endpoints cessent donc de partager ce DTO, `/collection/price-history` conservant son contrat actuel à
  l'identique.
- La résolution par `scryfall_id` cesse de retourner une finition. Elle doit devenir **déterministe** : plusieurs
  lignes de langues différentes partagent un `scryfall_id`, et un ordre explicite doit décider laquelle est retenue.
  Elles partagent aussi le `cardmarket_id`, seule donnée dont cet endpoint a besoin.
- Le contrôle d'existence de `/card/offers` porte sur l'identité de carte, qui ne comprend plus le foil.
  Conséquence assumée : une demande d'offres pour une finition qui n'existe pas pour cette impression retourne
  200 avec une liste vide, et non 404. C'est le comportement déjà prescrit par la spec `028` pour une carte que
  personne ne possède.
- Le contrat OpenAPI (`doc/openapi.yml`) et les clients générés — bindings TypeScript de `frontend-vue`, client
  Swift de `ios-app` — sont régénérés.

### Conséquence assumée sur le compteur de cartes

`stats_repository_adapter.rs:22` expose un `count(*)` sur `card`. La déduplication le fait mécaniquement baisser
(6 991 → 6 750 sur la base de développement) sans qu'aucune carte n'ait disparu du catalogue : ce compteur cessait
de compter des cartes pour compter des variantes. Le changement est assumé et doit être documenté ; aucune
compensation n'est introduite pour préserver l'ancienne valeur.

### Frontend web et iOS

Les écrans lisent le foil sur les mêmes champs des mêmes réponses. Trois points d'attention :

- Les appels à l'historique de prix (`DetailModal.vue` côté web, `CardDetailViewModel.swift` côté iOS) doivent
  transmettre la finition de la carte consultée. Elle leur est déjà disponible dans les deux cas.
- Le client iOS absorbe aujourd'hui un 400 de cet endpoint en « pas assez de données à tracer »
  (`CardDetailViewModel.swift:120`). Tant que ce traitement subsiste, un oubli du paramètre `foil` se manifesterait
  par un graphique vide plutôt que par une erreur. Ce traitement doit être revu pour que l'absence de paramètre
  requis ne soit pas confondue avec une absence de données.
- L'affichage distinctif du foil (badge `⭑` web, `FoilEffect` iOS) est conservé à l'identique.

### Tests

Les helpers d'insertion de cartes de `common_repository_tests.rs` prennent un argument `foil` et alimentent toutes
les suites d'intégration. Leur signature change, ce qui touche l'ensemble des préparations de jeux de données — pas
seulement les assertions. Cet impact est mécanique mais massif, et doit être traité comme tel : aucune assertion
fonctionnelle existante ne doit être relâchée pour faire passer une suite.

### Documentation

- `.agents/database-schema.instructions.md` doit être mis à jour à **deux** endroits, sans quoi il contredirait le
  schéma : l'invariant « Card identity » décrit comme un quadruplet incluant `foil` et son effet de propagation, et
  la description de `find_trade_cards_with_details` comme lisant `mv_last_cardmarket_prices` « joined on `card` ».
- `doc/db.md` est régénéré.

## Cas d'erreurs

- **Divergence réelle entre deux variantes d'une même carte** : si deux lignes de `card` partageant
  `(set_code, collector_number, language_code)` portent des valeurs **non nulles** différentes sur `cardmarket_id`
  ou `the_gatherer_id`, ou des `scryfall_id`, `name` ou `rarity` différents, l'hypothèse de fusion est fausse et la
  déduplication perdrait de l'information. La migration doit s'arrêter en signalant les clés concernées, et non
  choisir arbitrairement. Une valeur face à un `NULL` n'est pas une divergence (voir Solution).
- **Base vide ou de test** : la migration s'applique sans donnée, les vues sont recréées vides et le restent jusqu'au
  premier rafraîchissement. Aucun lecteur ne doit échouer dans cet état.
- **Appel à l'historique de prix par carte sans paramètre `foil`** : rejeté en 400 par le serveur, jamais traité avec
  une finition supposée. Ce rejet ne doit pas être absorbé côté client en « pas de données » (voir la section iOS).
- **Carte sans `cardmarket_id`** : les deux finitions sont présentes dans la vue des derniers prix avec des prix
  absents. Un prix absent ne doit jamais être substitué par zéro.
- **Demande de prix foil pour une impression jamais parue en foil** : le catalogue ne connaissant pas les finitions
  disponibles (hors périmètre, voir ci-dessous), aucune validation n'est faite. Les colonnes foil de
  `cardmarket_price` étant alors vides, le résultat est un prix absent — pas une erreur.
- **Import contenant la version normale et la version foil de la même carte** : les deux entrées de collection sont
  créées, une seule définition de carte l'est. L'import ne doit ni échouer sur un conflit d'insertion, ni fusionner
  les deux entrées.
- **Divergence de `cardmarket_id` entre deux langues d'une même carte** : ce cas d'erreur de la spec `028` reste
  entièrement en vigueur et **n'est pas couvert** par le contrôle de migration décrit ci-dessus, qui porte sur une
  clé différente (langue comprise). La clé unique de `mv_last_cardmarket_prices` restant
  `(set_code, collector_number, foil)`, une telle divergence continuerait de faire échouer chaque rafraîchissement.
  Écart mesuré aujourd'hui : 0.

### Hors périmètre

Le catalogue ne stocke pas les finitions réellement disponibles pour une impression (le champ `finishes` de
Scryfall). Aucune validation « cette carte existe-t-elle en foil ? » n'est introduite, ni côté API ni côté interface.

## Critères d'acceptance

**Schéma et migration**

- [ ] Après migration, la table `card` ne comporte plus de colonne `foil` et sa clé primaire est
      `(set_code, collector_number, language_code)`.
- [ ] Les clés étrangères de `collection_entry` et `trade_card` vers `card` portent sur trois colonnes et ne
      référencent plus `foil`.
- [ ] `collection_entry.foil` et `trade_card.foil` existent toujours, en `NOT NULL`, et restent dans la clé
      d'unicité de `collection_entry` et dans la clé primaire de `trade_card`.
- [ ] Étant donné une base contenant une carte présente en version foil **et** non-foil, quand la migration est
      appliquée, alors `card` ne contient plus qu'une ligne pour cette clé, et les colonnes `scryfall_id`,
      `cardmarket_id`, `the_gatherer_id`, `name` et `rarity` de la ligne survivante sont celles d'origine.
- [ ] Étant donné une carte dont la variante foil porte un `cardmarket_id` et la variante non-foil un `NULL`, quand
      la migration est appliquée, alors elle réussit et la ligne survivante porte ce `cardmarket_id`. Idem pour
      `the_gatherer_id`.
- [ ] Étant donné deux variantes d'une même clé portant des `cardmarket_id` **tous deux non nuls et différents**,
      quand la migration est appliquée, alors elle échoue en nommant la clé en cause et la base reste dans son état
      antérieur.
- [ ] Étant donné une base contenant une carte présente **uniquement** en version foil, quand la migration est
      appliquée, alors sa définition subsiste et les entrées de collection qui la référencent restent valides.
- [ ] Le nombre total de lignes de `collection_entry` et de `trade_card` est identique avant et après migration, et
      la valeur `foil` de chaque ligne est inchangée.
- [ ] La migration s'applique sans erreur sur une base vide.

**Domaine et import**

- [ ] L'identité de carte du domaine ne comporte plus de champ `foil`, et sa représentation textuelle n'affiche plus
      de symbole de finition.
- [ ] Étant donné un CSV contenant la même carte en `normal` et en `foil` **dans le même classeur**, quand il est
      importé, alors deux entrées de collection sont créées, de quantité 1 chacune, avec `foil` respectivement à
      `false` et `true` — et non une entrée unique de quantité 2.
- [ ] Étant donné ce même CSV, quand il est importé, alors `card` contient une seule ligne pour cette carte et
      l'import se termine sans erreur.
- [ ] Étant donné un CSV dont le champ de finition vaut `normal`, quand il est importé, alors l'entrée de collection
      créée porte `foil = false`.
- [ ] Étant donné un CSV dont le champ de finition vaut une autre valeur (`foil`, `etched`, …), quand il est
      importé, alors l'entrée de collection créée porte `foil = true`.
- [ ] Étant donné la même carte foil présente dans deux classeurs différents, quand elle est importée, alors deux
      entrées de collection distinctes sont créées, toutes deux `foil = true`.

**Prix et vues**

- [ ] `mv_last_cardmarket_prices` contient exactement deux lignes par `(set_code, collector_number)` distinct du
      catalogue, une par finition, et son index unique est créé.
- [ ] Pour une carte donnée, la ligne `foil = true` expose les colonnes `low_foil`, `trend_foil`, `avg_foil` du
      relevé de date maximale, et la ligne `foil = false` les colonnes `low`, `trend`, `avg` du même relevé.
- [ ] Une carte sans `cardmarket_id` est présente dans la vue pour les deux finitions, avec `low`, `trend` et `avg`
      absents — et non à zéro.
- [ ] Le contenu de `mv_card_prices` est strictement identique avant et après la migration, sur toutes ses colonnes
      et pour le même nombre de lignes.
- [ ] Étant donné un utilisateur possédant la version foil et la version non-foil de la même carte, quand
      `mv_card_prices` est rafraîchie, alors elle expose deux lignes portant des prix distincts, conformes à leur
      finition.
- [ ] Le contenu de `v_tradable_entry` est strictement identique avant et après la migration.
- [ ] Étant donné un utilisateur possédant la version foil et la version non-foil de la même carte, quand
      l'historique de valeur de collection est calculé pour une date, alors la valeur écrite dans
      `collection_price_history` est identique à celle produite avant la migration pour le même jeu de données.
- [ ] Étant donné un échange portant une carte foil, quand on consulte son détail, alors le prix affiché est le prix
      foil ; pour la même carte non-foil dans un autre échange, c'est le prix non-foil.
- [ ] Les statistiques de collection et les filtres de rareté retournent les mêmes résultats qu'avant la migration
      pour un même jeu de données.
- [ ] Un relevé de prix nouvellement inséré est visible depuis `mv_card_prices` après un seul cycle de
      rafraîchissement.

**API**

- [ ] Le paramètre `foil` est déclaré obligatoire pour `GET /card/{scryfall_id}/price-history` dans
      `doc/openapi.yml`, et un appel HTTP à cette route sans ce paramètre retourne 400.
- [ ] `GET /collection/price-history` accepte exactement les mêmes paramètres qu'avant la migration, et un appel
      sans paramètre de finition retourne 200.
- [ ] Étant donné une carte possédée en foil et en non-foil, quand on appelle
      `GET /card/{scryfall_id}/price-history` avec `foil=true` puis `foil=false`, alors les deux réponses sont 200
      et exposent des séries de prix différentes, correspondant respectivement aux colonnes foil et non-foil.
- [ ] La requête de résolution par `scryfall_id` impose un ordre explicite sur la clé de carte ; étant donné un
      `scryfall_id` présent en plusieurs langues, le `cardmarket_id` retourné est celui de la ligne désignée par cet
      ordre.
- [ ] `GET /card/{scryfall_id}/price-history` pour un `scryfall_id` inconnu retourne 404, quelle que soit la valeur
      de `foil`.
- [ ] Étant donné une carte sans `cardmarket_id`, quand on appelle `GET /card/{scryfall_id}/price-history`, alors
      la réponse est 200 avec une liste vide.
- [ ] Étant donné une carte existant en base, quand on appelle `GET /card/offers` avec `foil=true` alors que
      personne ne la possède en foil, alors la réponse est 200 avec une liste vide et `total: 0`, et non 404.
- [ ] Étant donné un `set_code`, `collector_number` et `language_code` ne correspondant à aucune carte, quand on
      appelle `GET /card/offers`, alors la réponse est 404.
- [ ] Les schémas `CollectionCardResponse`, `TradeCardResponse`, `AddTradeCardRequest`, `RemoveTradeCardRequest` et
      `CardOffersParams` exposent toujours `foil`, avec le même type et la même obligation qu'avant.
- [ ] `doc/openapi.yml` est régénéré ; ses seules différences portent sur les paramètres de l'historique de prix par
      carte et sur le découplage du DTO partagé avec `/collection/price-history`.

**Interfaces**

- [ ] Les bindings TypeScript et le client Swift sont régénérés et compilent.
- [ ] Le détail d'une carte foil affiche son badge `⭑` (web) et son effet foil (iOS), à l'identique de l'existant.
- [ ] Le graphique d'historique de prix d'une carte foil affiche la série foil, sur le web comme sur iOS.
- [ ] Côté iOS, une réponse 400 de l'historique de prix ne se traduit plus par un état « pas assez de données ».
- [ ] Ajouter puis retirer une carte foil d'un échange fonctionne sans affecter l'entrée non-foil correspondante.

**Qualité**

- [ ] Les suites de tests existantes de `/collection`, `/search/card`, `/collection/stats`, `/collection/price-history`
      et des échanges sont vertes, sans qu'aucune assertion fonctionnelle existante ait été supprimée ou relâchée.
- [ ] `.agents/database-schema.instructions.md` ne décrit plus `foil` comme partie de l'identité de carte, et sa
      description de `find_trade_cards_with_details` ne mentionne plus une jointure des prix via `card`.
- [ ] `doc/db.md` est régénéré.
- [ ] Le cache de requêtes hors-ligne est régénéré (`mise run sqlx-prepare`) et `mise run lint-sqlx` ne signale
      aucun écart.
- [ ] `mise run lint-backend` et `mise run format` passent sans erreur.
