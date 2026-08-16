# Spec : Rattachement des cartes à un binder ManaBox

## Contexte

Le CSV d'import ManaBox contient une colonne `Binder Name` (index 0) que le parseur
(`parse_service.rs`) lit déjà via le crate `csv` mais n'exploite pas — la valeur est
ignorée. La table `collection_entry` n'a aujourd'hui aucune notion de binder : sa clé
primaire est `(set_code, collector_number, language_code, foil, user_id)`, soit une seule
ligne par carte possédée par utilisateur.

Le besoin produit : à terme, un utilisateur pourra restreindre les cartes qu'il propose à
l'échange à celles rangées dans un binder donné (suite logique de la spec 017 sur la
visibilité de collection — le commentaire `frontend-vue/app/utils/trade-rules.ts:5`
anticipe déjà cette intégration : _"binders ManaBox viendront du backend"_). Un même
utilisateur peut posséder plusieurs exemplaires d'une même carte répartis dans des binders
différents (ex : un exemplaire dans un binder d'échange, un autre dans "bulk") — seul un
sous-ensemble sera un jour proposé à l'échange. Cela impose de distinguer les exemplaires
par binder, et non de les fusionner comme aujourd'hui.

## Objectif

Stocker, lors de l'import ManaBox, le binder d'origine de chaque exemplaire de carte, en
conservant une ligne `collection_entry` distincte par binder pour une même carte. Le
comportement actuellement visible (liste de collection, statistiques, recherche) ne doit
pas changer : une carte reste présentée comme une seule entrée par (carte, utilisateur),
même si elle est stockée en plusieurs lignes en base.

## Solution

### Import / parsing

- `parse_service.rs` extrait la colonne `Binder Name` (index 0) en `Option<String>`. Une
  valeur vide ou uniquement composée d'espaces est normalisée à `None`.
- La fusion des lignes dupliquées, aujourd'hui basée sur `CardId`, doit désormais se faire
  sur `(CardId, binder_name)` : deux lignes identiques par ailleurs mais avec un
  `Binder Name` différent ne sont plus fusionnées et produisent deux entrées distinctes.
  Deux lignes strictement identiques (`Binder Name` compris) restent fusionnées comme
  aujourd'hui : quantité sommée, prix d'achat moyenné pondéré par les quantités, `added_at`
  le plus ancien conservé.
- **Contrainte** : `CardId` est l'identité de la carte, partagée avec la table `card` (sa
  clé primaire) et avec `trade_card` — `binder_name` ne doit **pas** y être ajouté. C'est
  une propriété de l'entrée de collection, pas de la carte.
- `import_card_service.rs` conserve son comportement de remplacement complet (suppression
  de toutes les entrées de l'utilisateur via `delete_all`, puis réinsertion depuis le CSV).

### Modèle de données

- Ajouter une colonne `binder_name` (`VARCHAR`, nullable, sans valeur par défaut) à
  `collection_entry`.
- `binder_name` doit distinguer deux entrées d'une même carte pour un même utilisateur, y
  compris quand sa valeur est nulle (un utilisateur peut avoir un exemplaire "sans binder"
  et un exemplaire dans un binder nommé, pour la même carte).
- **Conséquence** : la clé primaire actuelle ne peut pas accueillir une colonne nullable
  (PostgreSQL interdit les `NULL` dans une PK). L'unicité doit donc être portée par une
  contrainte qui traite `NULL` comme une valeur à part entière — PostgreSQL 18 (version
  utilisée par le projet) le permet nativement. Le choix exact (contrainte, index) est
  laissé au plan d'implémentation, mais la cible `ON CONFLICT` de l'upsert de
  `card_repository_adapter.rs::save` doit rester valide et inclure `binder_name`.

### Cohérence des lectures existantes

Une même carte peut désormais correspondre à plusieurs lignes `collection_entry` pour un
utilisateur. Les lectures existantes doivent continuer à présenter une carte comme une
seule entrée par (carte, utilisateur), avec ces règles d'agrégation :

- **quantité** : somme des lignes de la carte, plafonnée à 255 (le domaine expose
  `quantity` en `u8` ; le plafonnement doit être explicite, jamais un débordement
  silencieux).
- **prix d'achat** : moyenne pondérée par les quantités, cohérente avec la fusion faite à
  l'import.
- **`added_at`** : la plus ancienne des lignes, cohérente avec la fusion faite à l'import.

Points à corriger :

- **`mv_card_prices`** : la vue fait aujourd'hui une jointure à plat sur `collection_entry`
  et porte un index unique sur `(set_code, collector_number, language_code, foil,
user_id)`. Sans agrégation, le `REFRESH` échouerait sur violation de clé unique dès
  qu'une carte est répartie sur deux binders — l'import serait cassé. La vue doit donc
  agréger les lignes par (carte, utilisateur) selon les règles ci-dessus, et son index
  unique doit rester valide.
- **`card_repository_adapter.rs::get_all`** : jointure à plat sur `collection_entry`, doit
  agréger selon les mêmes règles.
- **`collection_stats_repository_adapter.rs`** : `unique_cards` est calculé en `COUNT(*)`
  sur `collection_entry` — doit compter les cartes distinctes
  (`set_code`, `collector_number`, `language_code`, `foil`) et non le nombre de lignes.
  `total_cards` (`SUM(quantity)`) reste correct.
- **`trade_repository_adapter.rs::find_collection_entry_quantity`** : lit la quantité
  possédée d'une carte pour valider qu'un utilisateur détient assez d'exemplaires avant de
  les ajouter à un échange. La requête ne lit aujourd'hui qu'une seule ligne : elle doit
  retourner le **total possédé, tous binders confondus**, sinon un utilisateur dont les
  exemplaires sont répartis sur plusieurs binders se verrait refuser à tort l'ajout de sa
  quantité réelle. Une carte non possédée doit continuer à être distinguée d'une quantité
  nulle, pour préserver l'erreur fonctionnelle existante.

En revanche, la liste paginée de collection et le filtre de recherche `owned=true` lisent
tous deux `mv_card_prices` : corriger la vue les corrige automatiquement, sans changement
de leurs requêtes. L'autocomplétion d'utilisateurs et l'historique de valeur de collection
somment déjà sur toutes les lignes de `collection_entry` et restent corrects sans
modification. Aucun changement frontend n'est nécessaire, l'API continuant à retourner une
entrée par carte.

`binder_name` lui-même n'est exposé nulle part (ni API, ni frontend) dans cette spec —
uniquement stocké, et pris en compte en interne pour préserver l'agrégation existante.

## Cas d'erreurs

- **`Binder Name` vide ou absent sur une ligne** → `binder_name = null` pour l'entrée
  correspondante, import non bloqué.
- **CSV au format "export de binder"** (15 colonnes, déjà rejeté avec l'erreur _"expecting
  a collection export, got a binder export"_) → comportement inchangé.
- **Quantité agrégée supérieure à 255** (carte répartie sur plusieurs binders avec de
  grandes quantités) → valeur plafonnée à 255, sans débordement silencieux.
- **Quantité totale nulle** sur une carte (toutes ses lignes à 0) → le calcul de la moyenne
  pondérée du prix d'achat ne doit pas provoquer de division par zéro ; prix d'achat à 0.
- **Ré-import** : le remplacement complet existant supprime toutes les entrées de
  l'utilisateur avant réinsertion. Une carte présente lors d'un import précédent mais
  absente du nouveau CSV (ou déplacée dans un autre binder) ne subsiste donc pas en base —
  elle n'est plus comptée comme possédée, et ne laisse aucune ligne orpheline.

## Critères d'acceptance

- [ ] Une migration ajoute `collection_entry.binder_name` (`VARCHAR` nullable, sans valeur
      par défaut) et adapte la contrainte d'unicité pour y inclure `binder_name` en
      traitant `NULL` comme une valeur distinctive.
- [ ] Given un CSV ManaBox avec une valeur non vide dans `Binder Name` pour une carte,
      When j'importe, Then `collection_entry.binder_name` vaut cette valeur pour l'entrée
      créée.
- [ ] Given un CSV ManaBox avec un `Binder Name` vide ou composé d'espaces, When
      j'importe, Then `collection_entry.binder_name` est `NULL` pour l'entrée créée.
- [ ] Given un CSV avec deux lignes pour la même carte (même `set_code`,
      `collector_number`, `language_code`, `foil`) mais des `Binder Name` différents, When
      j'importe, Then deux lignes `collection_entry` distinctes sont créées pour cet
      utilisateur, une par binder, avec leurs quantités respectives.
- [ ] Given un CSV avec deux lignes pour la même carte, l'une avec un `Binder Name`
      renseigné et l'autre vide, When j'importe, Then deux lignes distinctes sont créées
      (une avec `binder_name` renseigné, une avec `NULL`).
- [ ] Given un CSV avec deux lignes strictement identiques, `Binder Name` compris, When
      j'importe, Then une seule ligne `collection_entry` est créée avec la quantité sommée
      (comportement actuel inchangé).
- [ ] Given un utilisateur possédant une même carte répartie sur 2 binders, When j'appelle
      l'endpoint de récupération de la collection, Then la carte apparaît une seule fois,
      avec la quantité totale, le prix d'achat moyen pondéré et le `added_at` le plus
      ancien des deux lignes.
- [ ] Given le même contexte, When je consulte les statistiques de collection, Then
      `unique_cards` compte cette carte une seule fois et `total_cards` inclut la somme des
      deux binders.
- [ ] Given le même contexte, When une recherche avec `owned=true` est effectuée par cet
      utilisateur, Then la carte apparaît une seule fois dans les résultats.
- [ ] Given un utilisateur possédant 2 exemplaires d'une carte dans un binder et 3 dans un
      autre, When il ajoute 5 exemplaires de cette carte à un échange, Then l'ajout est
      accepté.
- [ ] Given un utilisateur ne possédant pas une carte, When il tente de l'ajouter à un
      échange, Then l'ajout est refusé (comportement actuel inchangé).
- [ ] Given un import où une carte est répartie sur 2 binders, When l'import se termine,
      Then le `REFRESH` de `mv_card_prices` réussit et la vue contient exactement une ligne
      pour cette carte et cet utilisateur.
- [ ] Given une carte présente en base après un premier import, When je ré-importe un CSV
      qui ne la contient plus, Then elle n'est plus présente dans `collection_entry` pour
      cet utilisateur et n'apparaît plus dans sa collection ni dans ses statistiques.
- [ ] Un import complet d'un des fichiers de `example-files/` s'exécute sans erreur et les
      `binder_name` en base correspondent aux valeurs du CSV.
- [ ] `mise run checks` passe sans erreur (inclut `rebuild-db-doc`, `sqlx-prepare`, tests
      et lint).

## Hors scope

- Exposition de `binder_name` via l'API ou le frontend (`CollectionEntryResponse`,
  `doc/openapi.yml`, écran collection) — prévu dans une spec ultérieure de filtrage par
  binder pour la mise à l'échange.
- Filtrage effectif des cartes proposées à l'échange selon leur binder (règles de mise à
  l'échange côté profil, `TradeRules.vue`) — objet d'une spec de suivi dédiée.
- Gestion/CRUD des binders eux-mêmes (renommage, suppression, liste) — ManaBox reste la
  source de vérité, `binder_name` est une donnée importée en lecture seule.
- Atomicité de l'import (le `delete_all` suivi des insertions n'est pas transactionnel
  aujourd'hui) — comportement pré-existant, non modifié par cette spec.
