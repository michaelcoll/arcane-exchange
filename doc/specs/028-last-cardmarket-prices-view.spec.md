# Spec : Vue matérialisée des derniers prix Cardmarket

## Contexte

Récupérer « le dernier prix Cardmarket connu d'une carte » impose aujourd'hui de recalculer, à chaque requête, un
agrégat `MAX(date)` par produit sur la table `cardmarket_price` (4,4 M lignes, 35 dates distinctes en base de
développement). Ce motif est écrit en clair dans `find_trade_cards_with_details`
(`trade_repository_adapter.rs:164`), qui le paie à chaque consultation du détail d'un échange.

La vue matérialisée existante `mv_card_prices` contient déjà ce motif, mais elle est jointe à `collection_entry` :
elle ne connaît que les cartes **possédées** par au moins un utilisateur, et produit une ligne par (carte,
utilisateur). Elle est donc inutilisable dès qu'on a besoin d'un prix indépendamment de la possession — un test
existant le documente explicitement
(`find_trade_cards_with_details_survives_owner_removing_collection_entry`, `trade_repository_adapter.rs:1998`).
C'est la raison pour laquelle la requête des échanges duplique le motif au lieu de réutiliser la vue.

La migration `0024_add_last_cardmarket_prices.sql` introduit `mv_last_cardmarket_prices`, qui comble ce manque :
dernier prix par carte, sans condition de possession. Elle n'est à ce jour ni rafraîchie, ni consommée par le code,
et n'est pas encore intégrée au dépôt — cette spec suppose son intégration en l'état.

Mesures relevées sur la base de développement, qui cadrent l'enjeu :

| Situation                                                 | Temps   |
| --------------------------------------------------------- | ------- |
| `find_trade_cards_with_details`, motif recalculé (actuel) | 282 ms  |
| Même requête adossée à la vue                             | 0,04 ms |
| `REFRESH` de `mv_card_prices` (payé à chaque import)      | 370 ms  |

## Objectif

Disposer d'une source pré-calculée du dernier prix Cardmarket par carte, accessible sans exiger que la carte soit
possédée, et l'utiliser là où ce prix est aujourd'hui recalculé à chaque requête ou faussé par une condition de
possession. Cette vue devient le seul endroit où l'agrégat « dernier relevé par produit » est calculé :
`mv_card_prices`, qui le recalculait à l'identique, s'y adosse. Elle reste la source du prix pour la collection, la
recherche et les statistiques, mais cesse de faire ce calcul elle-même.

## Solution

### Forme et grain de la vue

- La vue est clé par `(set_code, collector_number, foil)` et expose les prix `low`, `trend` et `avg` déjà résolus
  selon la finition (colonnes foil ou non-foil).
- `language_code` est délibérément absent : le prix Cardmarket ne dépend pas de la langue. Cette exclusion est
  légitime **parce que** deux lignes de `card` ne différant que par la langue partagent le même `cardmarket_id`, et
  produisent donc des lignes strictement identiques que le `DISTINCT` écrase sans perte d'information (vérifié :
  41 clés concernées, 0 divergence de `cardmarket_id`).
- Un index unique sur la clé est obligatoire : c'est la condition d'un rafraîchissement concurrent.
- **Conséquence à respecter** : cette vue ne peut pas servir à décider qu'un `CardId` existe, puisqu'elle ne
  discrimine pas la langue. Voir la section `/card/offers`.

### Rafraîchissement

Le contenu de la vue dépend de **deux** sources : les relevés de `cardmarket_price`, et la table `card` — dont la
colonne `cardmarket_id` est renseignée après coup, par un flux distinct de l'import de prix
(`card_repository_adapter.rs:145`).

- La vue doit donc être rafraîchie à chacun des flux qui modifient l'une de ces sources, c'est-à-dire aux **mêmes
  quatre points** où `mv_card_prices` l'est déjà : import de prix, import de cartes, résolution des identifiants
  Cardmarket, et mise à jour Gatherer. Ne la rafraîchir qu'à l'import de prix laisserait une carte fraîchement
  importée, ou dont le `cardmarket_id` vient d'être résolu, absente de la vue jusqu'au cycle de prix suivant —
  soit un prix manquant là où le code actuel en affiche un.
- Le rafraîchissement s'exécute en mode concurrent, afin de ne pas bloquer les lectures.
- `mv_card_prices` étant adossée à cette vue (voir section suivante), **l'ordre est contraignant** : la vue des
  derniers prix doit être rafraîchie en premier. L'ordre inverse peuplerait `mv_card_prices` à partir de prix
  vieux d'un cycle, sans aucune erreur visible.
- La vue doit être peuplée une première fois avant tout rafraîchissement concurrent.

### Traitement des échecs de rafraîchissement

Les quatre points d'appel ne traitent pas l'erreur de la même façon aujourd'hui : l'import de prix la **propage**
(`import_price_service.rs:38`), les flux Cardmarket et Gatherer la tracent sans interrompre le traitement
(`update_card_market_service.rs:77`). Cette spec ne change pas le comportement existant de `mv_card_prices` ; pour
la nouvelle vue, la règle est :

- Un échec de rafraîchissement ne doit pas invalider les données déjà écrites (prix ou cartes enregistrés).
- Il doit être **remonté au système d'observabilité**, pas seulement journalisé, car son seul symptôme visible
  serait sinon une vue figée servant des prix périmés indéfiniment.
- Le cycle suivant retente le rafraîchissement. Un échec qui persiste sur plusieurs cycles traduit une anomalie de
  données (voir Cas d'erreurs) et doit rester visible jusqu'à résolution.

### Bascule du détail des échanges

- `find_trade_cards_with_details` cesse de recalculer le motif et lit la vue.
- L'invariant fonctionnel à préserver est celui que garantit le test existant : une carte reste visible dans le
  détail d'un échange même si son propriétaire l'a retirée de sa collection.
- Ce qui change en contrepartie, et qui est assumé : le prix affiché n'est plus lu en temps réel mais provient du
  dernier rafraîchissement. C'est le compromis accepté de la matérialisation.
- Une carte sans prix Cardmarket connu continue de remonter avec un prix absent, jamais avec un prix nul.
- **Impact sur les tests** : les tests d'intégration créent leur jeu de données après les migrations, sur une base
  où la vue a été peuplée à vide. Tout test lisant un prix par cette voie devra donc rafraîchir la vue lors de sa
  phase de préparation. Le helper de test existant, qui ne rafraîchit que `mv_card_prices`
  (`common_repository_tests.rs:454`), doit être étendu.

### Correction du contrôle d'existence de `/card/offers`

- Le contrôle d'existence qui précède la recherche d'offres s'appuie sur `mv_card_prices` sans filtre utilisateur
  (`card_prices_view_repository_adapter.rs:320`) : « la carte existe » y signifie en réalité « au moins un
  utilisateur la possède ».
- La spec 007 est elle-même ambiguë sur ce point : elle prescrit `mv_card_prices` comme source de résolution
  (section Solution) et exige un 404 quand aucune ligne n'y correspond, tout en demandant par ailleurs un 200 avec
  liste vide pour une carte existante que personne d'autre ne possède. **Cette spec amende la 007** : la
  résolution d'existence cesse d'être adossée à une source conditionnée à la possession.
- **Invariant impératif** : le contrôle d'existence doit rester discriminant sur les **quatre** attributs de
  `CardId`, `language_code` compris. La vue des derniers prix ne portant que trois colonnes de clé, elle ne peut
  pas tenir ce rôle seule ; la résolution doit s'appuyer sur une source qui connaît la langue.
- Le défaut corrigé est aujourd'hui **latent** : la table `card` n'étant alimentée qu'aux imports de collection,
  aucune carte en base n'est actuellement dépossédée (écart mesuré : 0). Il se manifesterait dès qu'une entrée de
  collection est supprimée, ou si le catalogue venait à être alimenté indépendamment des collections.

### Adosser `mv_card_prices` à la nouvelle vue

`mv_card_prices` recalcule aujourd'hui le même agrégat `MAX(date)` que la nouvelle vue. Les deux étant désormais
rafraîchies aux mêmes quatre points, laisser ce calcul en double ferait passer le coût d'un cycle de
rafraîchissement de 307 ms à environ 613 ms. `mv_card_prices` doit donc lire la nouvelle vue au lieu de recalculer
son propre agrégat : son rafraîchissement tombe alors de 307 ms à 14 ms, ramenant le cycle complet à ~320 ms.

- Seule la source des prix change. Toutes les autres colonnes de `mv_card_prices` — dont l'agrégation des entrées
  de collection par utilisateur — et l'intégralité de ses index sont conservées à l'identique.
- Une vue matérialisée ne se redéfinit pas en place : conformément aux migrations forward-only du projet, elle est
  supprimée et recréée dans la même migration, index compris.
- Le contenu de `mv_card_prices` doit rester strictement identique avant et après ce changement : c'est une
  optimisation du coût de rafraîchissement, jamais une évolution de ce qu'elle expose.
- Contrepartie assumée : l'ordre de rafraîchissement devient significatif (voir section précédente).

### Piste documentée, hors périmètre : `mv_card_prices` en vue simple

Une fois le `MAX(date)` matérialisé, la matérialisation de `mv_card_prices` elle-même ne rapporte plus que ~7 ms
en lecture (1,0 ms contre 7,8 ms pour une vue non matérialisée), alors qu'elle impose un rafraîchissement
synchrone à chaque import de collection (`import_card_service.rs:70`) et des données de collection périmées entre
deux cycles. La dématérialiser supprimerait ce coût et rendrait la collection immédiatement à jour.

Cette piste **n'est pas engagée par cette spec** : les mesures viennent d'une base de développement (7 151 entrées
de collection) et doivent être revalidées sur un volume réaliste avant toute décision.

## Cas d'erreurs

- **Vue jamais peuplée** : le rafraîchissement concurrent échoue tant que la vue n'a pas été peuplée une première
  fois. Le premier peuplement doit être garanti par la migration elle-même.
- **Base neuve ou environnement de test** : la migration s'applique sur une base vide, la vue est créée à zéro
  ligne et le reste jusqu'au premier rafraîchissement. Tout lecteur doit se comporter correctement dans cet état —
  prix absent, jamais d'erreur.
- **Divergence de `cardmarket_id` entre deux langues d'une même carte** : le `DISTINCT` ne dédoublonnerait plus,
  la clé unique serait violée et le rafraîchissement échouerait à chaque cycle. Rien ne l'empêche
  structurellement, le `cardmarket_id` étant écrit par carte et par langue. Cet échec ne doit pas être avalé
  silencieusement : c'est le seul signal d'une anomalie qui, sinon, fige la vue sur des prix périmés.
- **Carte sans `cardmarket_id`, ou produit sans relevé de prix** : les trois prix sont absents. Aucun consommateur
  ne doit substituer une valeur par défaut — un prix absent se distingue d'un prix à zéro.
- **Carte présente au catalogue mais possédée par personne** : elle doit rester consultable et son existence
  reconnue, et non traitée comme inexistante.
- **Rafraîchissements concurrents** : deux flux peuvent déclencher un rafraîchissement simultané de la même vue.
  Le comportement attendu est que l'un attende l'autre sans échec ni interblocage.
- **Ordre de rafraîchissement inversé** : rafraîchir `mv_card_prices` avant la vue des derniers prix la peuple à
  partir du cycle précédent. Aucune erreur n'est levée et le symptôme est un prix systématiquement en retard d'un
  cycle sur la collection et la recherche. C'est le principal risque de régression silencieuse de cette spec, et il
  doit être couvert par un test.

## Critères d'acceptance

- [ ] Appliquée sur une base contenant au moins une clé `(set_code, collector_number, foil)` présente en plusieurs
      langues, la migration `0024` s'applique sans erreur et l'index unique est créé.
- [ ] La vue contient exactement une ligne par `(set_code, collector_number, foil)` distinct de la table `card`,
      sans doublon.
- [ ] Pour une carte donnée, les prix exposés par la vue sont ceux du relevé `cardmarket_price` de date maximale
      pour son `cardmarket_id`.
- [ ] Pour une carte foil, les prix exposés sont les colonnes foil ; pour une non-foil, les colonnes non-foil.
- [ ] Une carte sans `cardmarket_id` est présente dans la vue avec `low`, `trend` et `avg` absents.
- [ ] Deux cartes ne différant que par `language_code` produisent une seule ligne, portant les mêmes prix.
- [ ] Après un import de prix, un relevé plus récent inséré est reflété par la vue sans intervention manuelle
      (vérifié par un test d'intégration, pas seulement par l'appel au rafraîchissement).
- [ ] Après un import de cartes, une carte nouvellement insérée est présente dans la vue sans intervention
      manuelle.
- [ ] Après la résolution d'un `cardmarket_id` sur une carte qui n'en avait pas, la vue expose les prix
      correspondants sans intervention manuelle.
- [ ] Le rafraîchissement de la vue est émis en mode concurrent.
- [ ] Si le rafraîchissement de la nouvelle vue échoue, les prix et cartes déjà enregistrés le restent, et l'échec
      est remonté au système d'observabilité.
- [ ] `find_trade_cards_with_details` ne contient plus d'agrégat `MAX(date)` sur `cardmarket_price`.
- [ ] Le plan d'exécution de la requête du détail d'un échange ne référence plus la table `cardmarket_price` et ne
      comporte plus de nœud d'agrégation ; l'accès aux prix s'y fait par parcours de l'index unique de la vue.
- [ ] Étant donné un échange dont le propriétaire d'une carte a supprimé son entrée de collection, quand on
      consulte le détail de l'échange, alors la carte est toujours retournée avec son prix.
- [ ] Le détail d'un échange portant sur une carte sans prix Cardmarket retourne un prix absent, et non un prix à
      zéro.
- [ ] Étant donné une carte présente en base que personne ne possède, quand on appelle `/card/offers` pour cette
      carte, alors la réponse est 200 avec une liste vide et `total: 0`, et non 404.
- [ ] Étant donné un `CardId` dont les `set_code`, `collector_number` et `foil` existent en base mais dont le
      `language_code` ne correspond à aucune carte, quand on appelle `/card/offers`, alors la réponse est 404.
- [ ] Étant donné un `CardId` qui ne correspond à aucune carte en base, quand on appelle `/card/offers`, alors la
      réponse est 404.
- [ ] Après l'adossement, le contenu de `mv_card_prices` est strictement identique à sa version précédente, sur
      toutes ses colonnes et pour le même nombre de lignes.
- [ ] `mv_card_prices` ne contient plus d'agrégat `MAX(date)` sur `cardmarket_price`, et tous ses index sont
      recréés à l'identique.
- [ ] Un relevé de prix nouvellement inséré est visible depuis `mv_card_prices` après **un seul** cycle de
      rafraîchissement — ce qui atteste que l'ordre est respecté.
- [ ] Les suites de tests existantes de `/collection`, `/search/card` et `/collection/stats` passent sans
      modification de leurs assertions.
- [ ] Le cache de requêtes hors-ligne est régénéré (`mise run sqlx-prepare`) et `mise run lint-sqlx` ne signale
      aucun écart.
- [ ] `mise run lint-backend` et `mise run format` passent sans erreur.
