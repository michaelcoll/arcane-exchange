# Spec : Application effective des filtres d'échange

## Contexte

Trois specs successives ont mis en place les réglages d'échange de l'écran profil, chacune en excluant
explicitement de son périmètre leur application réelle :

- **017** — `users.visibility` (`public` / `trade` / `private`, défaut `private`), exposée par
  `GET`/`PUT /user/visibility`.
- **019** — `trading_binders` : les binders ManaBox ouverts à l'échange, exposés par `/user/trade-binders`.
- **020** — `collection_rarity_filters` : par rareté, le drapeau « ouverte à l'échange » et le nombre
  d'exemplaires gardés, exposés par `/collection/visibility/rarities`.

Aujourd'hui ces trois réglages ne sont que stockés et affichés. `GET /search/card`, `GET /card/offers` et
`GET /autocomplete/users` exposent l'intégralité des collections de tous les utilisateurs, y compris ceux
restés en `private` (soit, par défaut, tout le monde). Un joueur qui décoche un binder ou ferme une rareté ne
constate aucun changement, et `POST /trades/{trade_id}/cards` accepte n'importe quelle carte possédée par
n'importe qui.

## Objectif

Faire respecter ces trois réglages partout où les cartes d'un joueur sont exposées à un tiers ou engagées dans
un échange : recherche, offres, autocomplete joueur, ajout de cartes à un échange.

## Solution

### Notion centrale : la quantité proposée

Une règle unique, définie une fois et réutilisée par tous les endpoints concernés. Pour un couple
(utilisateur propriétaire, carte identifiée par `set_code` / `collector_number` / `language_code` / `foil`),
la **quantité proposée** vaut :

- `visibility = private` → **0**. L'utilisateur ne propose rien et n'apparaît nulle part.
- `visibility = public` → la **quantité totale possédée**, tous binders confondus, y compris les entrées sans
  binder (`binder_name` NULL). Les réglages de binders et de raretés sont ignorés.
- `visibility = trade` → le périmètre de la spec 020 : pour chaque entrée de collection rangée dans un binder
  coché (`trading_binders`), si la rareté de la carte est ouverte, `max(0, quantité − exemplaires gardés)` ;
  sinon 0. Les entrées hors binder coché et les entrées sans binder ne comptent jamais. L'absence de ligne
  dans `collection_rarity_filters` vaut « rareté fermée », donc 0.

Une quantité proposée nulle est indistinguable, pour un tiers, de l'absence de la carte dans la collection.

Cette règle porte sur la **lecture par un tiers**. Elle ne s'applique jamais à la vision qu'un utilisateur a
de sa propre collection : `GET /collection`, `GET /collection/stats`,
`GET /collection/visibility/rarities` et `GET /collection/price-history` restent inchangés.

### Contrainte d'implémentation

`mv_card_prices`, source unique de la recherche et des offres, agrège aujourd'hui `collection_entry` par
(carte, utilisateur) et **perd `binder_name`** au passage (cf. migration `0019`). La quantité proposée ne peut
donc pas être calculée à partir de la vue dans son état actuel : la vue — ou la source qu'elle agrège — doit
porter cette information. Le choix entre faire porter la quantité proposée par `mv_card_prices` et l'obtenir
par jointure au moment de la lecture appartient au plan d'implémentation ; la contrainte est que le résultat
reste cohérent avec le comptage « Proposés » déjà retourné par `GET /collection/visibility/rarities` (même
définition, mêmes exclusions), et que le rafraîchissement de la vue après import reste correct.

Les réglages de visibilité, binders et raretés changent en dehors des imports : si la quantité proposée est
matérialisée, elle doit refléter ces changements sans attendre le prochain import ManaBox.

### `GET /search/card`

- Seules les cartes dont **au moins un utilisateur** a une quantité proposée ≥ 1 apparaissent dans les
  résultats. Une carte que plus personne ne propose disparaît de la recherche.
- `owner_count` compte les utilisateurs **distincts** dont la quantité proposée pour cette carte est ≥ 1.
- `total` reflète le nombre de cartes uniques restantes après filtrage.
- Le filtre `player_username` — seul moyen aujourd'hui de consulter la collection d'un tiers — est soumis à la
  même règle : il ne retourne que les cartes réellement proposées par ce joueur. Cibler un joueur `private`
  retourne une page vide.
- Les autres filtres (`q`, `rarity`, `sets`, `price_min`, `price_max`), le tri et la pagination sont
  inchangés, et s'appliquent après ce filtrage.
- Le format de la réponse est inchangé. En particulier `quantity` reste masqué (0) en recherche publique.

### `GET /card/offers`

- Seuls les utilisateurs dont la quantité proposée pour la carte demandée est ≥ 1 apparaissent dans la liste.
  L'exclusion de l'utilisateur authentifié reste en place.
- Le champ `quantity` de chaque offre devient la **quantité proposée**, et non plus la quantité totale
  possédée. C'est un changement de sémantique à un champ existant, sans changement de forme de la réponse.
- Le drapeau `reserved` et le tri par `selling_price` sont inchangés : une carte engagée dans un trade
  `ONE_ACCEPTED` / `FULLY_ACCEPTED` continue d'être listée avec `reserved: true`.
- Une carte qui existe mais que plus personne ne propose retourne 200 avec une liste vide et `total: 0` — pas 404. Le 404 reste réservé au `CardId` qui ne correspond à aucune carte.

### `GET /autocomplete/users`

- Les utilisateurs en `private` ne sont plus jamais suggérés.
- Les utilisateurs dont le total des quantités proposées est nul (aucun binder coché, toutes raretés fermées,
  collection vide) ne sont plus suggérés non plus : suggérer un joueur qui ne propose rien mène à une
  recherche systématiquement vide.
- `card_count` devient la **somme des quantités proposées** sur toute la collection du joueur, et non plus le
  total de sa collection.

### `POST /trades/{trade_id}/cards`

Un échange ne peut porter que sur des cartes réellement proposées : sans cette validation, un client
contournant la recherche pourrait demander une carte d'un binder non coché, d'une rareté fermée ou d'un joueur
`private`.

`POST /trades` ne transporte aucune carte (il ne porte que le destinataire) : la validation n'a de sens que sur
l'ajout d'une carte à un trade existant, qui traite une carte à la fois.

- Quand la carte ajoutée appartient à **l'autre partie**, la quantité proposée par son propriétaire doit
  couvrir la quantité demandée. Sinon, refus et aucune écriture en base. Le refus réutilise le comportement
  déjà en place lorsque le propriétaire ne possède pas assez d'exemplaires — une carte non proposée est, du
  point de vue du demandeur, une carte introuvable : **404**, sans révéler les réglages du propriétaire.
- Les cartes que l'appelant met de **son propre côté** ne sont pas soumises à cette validation : il dispose
  librement de sa collection, la vérification reste celle de la quantité qu'il possède.
- Les validations existantes (existence de la carte, appartenance au trade, statut modifiable, carte déjà
  réservée ailleurs) sont conservées et inchangées.
- `POST /trades/{trade_id}/cards/remove` n'est pas concerné : retirer une carte doit rester possible même si
  elle a cessé d'être proposée entre-temps.
- La vérification porte sur l'état au moment de l'appel. Un trade dont les cartes cessent d'être proposées
  n'est pas invalidé rétroactivement (voir Hors scope).

### Architecture

Hexagonale, sans nouvelle table ni nouvel endpoint. La règle de quantité proposée est portée par le domaine et
consommée par les repositories existants (`card_prices_view_repository_adapter` pour la recherche et les
offres, `user_repository_adapter` pour l'autocomplete, la chaîne trade pour la validation), afin qu'une
évolution future de la règle n'ait qu'un seul point de modification.

### Documentation

Mettre à jour `doc/openapi.yml` : la sémantique de `quantity` dans `/card/offers`, celle de `owner_count` dans
`/search/card` et celle de `card_count` dans `/autocomplete/users`, ainsi que le nouveau cas de 404 de
`POST /trades/{trade_id}/cards`. `doc/db.md` est régénéré (`mise run rebuild-db-doc`).

## Cas d'erreurs

- **Tous les utilisateurs sont en `private`** (état par défaut du parc actuel) → `GET /search/card` retourne
  200 avec une page vide, `total: 0` ; l'écran de recherche affiche son état vide existant, sans erreur.
- **Carte existante mais plus proposée par personne** → `GET /card/offers` retourne 200, liste vide,
  `total: 0`.
- **`player_username` ciblant un joueur `private` ou inexistant** → 200 avec une page vide, sans distinction
  entre les deux cas : on ne révèle pas l'existence d'un compte privé.
- **Utilisateur en `trade` sans aucun binder coché** → il ne propose rien ; il n'apparaît ni dans la
  recherche, ni dans les offres, ni dans l'autocomplete.
- **Quantité gardée supérieure ou égale à la quantité possédée** → quantité proposée 0, l'utilisateur
  n'apparaît pas pour cette carte.
- **`POST /trades/{trade_id}/cards` sur une carte non proposée par son propriétaire** → 404, aucune ligne
  `trade_card` ajoutée, trade inchangé.
- **`POST /trades/{trade_id}/cards` avec une quantité supérieure à la quantité proposée** → 404, même
  comportement que la quantité insuffisante déjà en place aujourd'hui.
- **Token absent ou invalide** → 401, comportement inchangé sur tous les endpoints concernés.
- **Erreur base de données** → 500 (pattern existant).

## Critères d'acceptance

- [ ] Given un utilisateur B en `private` possédant une carte, When A la cherche via `GET /search/card`, Then
      la carte n'apparaît pas dans les résultats (ou son `owner_count` ne compte pas B si un autre joueur la
      propose).
- [ ] Given un utilisateur B en `public` possédant 3 exemplaires d'une carte répartis dans un binder non coché
      et hors binder, When A appelle `GET /card/offers` pour cette carte, Then B apparaît avec `quantity: 3`.
- [ ] Given un utilisateur B en `trade` ayant coché « Trade Binder » et ouvert la rareté `R` avec 1 exemplaire
      gardé, possédant 3 exemplaires d'une carte rare dans ce binder, When A appelle `GET /card/offers`, Then
      B apparaît avec `quantity: 2`.
- [ ] Given le même B avec la rareté `R` fermée, When A appelle `GET /card/offers`, Then B n'apparaît pas et
      la réponse est 200 avec `total: 0`.
- [ ] Given le même B dont les 3 exemplaires sont dans un binder non coché, When A appelle `GET /card/offers`,
      Then B n'apparaît pas.
- [ ] Given un B en `trade` dont les exemplaires ont `binder_name` NULL, When A appelle `GET /card/offers`,
      Then B n'apparaît pas.
- [ ] Given B en `trade` possédant 2 exemplaires d'une carte et gardant 2 exemplaires de sa rareté, When A
      appelle `GET /card/offers`, Then B n'apparaît pas (quantité proposée 0, jamais négative).
- [ ] Given trois joueurs possédant la même carte dont un seul la propose, When A appelle
      `GET /search/card`, Then la carte apparaît une fois avec `owner_count: 1`.
- [ ] Given une carte que plus aucun joueur ne propose, When A appelle `GET /search/card` avec un `q` qui la
      cible, Then elle n'apparaît pas dans les résultats et n'est pas comptée dans `total`.
- [ ] Given un parc où tous les utilisateurs sont en `private`, When A appelle `GET /search/card`, Then la
      réponse est 200 avec une page vide et `total: 0`.
- [ ] Given un joueur B en `trade` proposant des cartes, When A appelle
      `GET /search/card?player_username=B`, Then seules les cartes réellement proposées par B sont
      retournées.
- [ ] Given un joueur B en `private`, When A appelle `GET /search/card?player_username=B`, Then la réponse est
      200 avec une page vide, identique au cas d'un username inexistant.
- [ ] Given un utilisateur authentifié possédant une carte, When il appelle `GET /card/offers` pour cette
      carte, Then il n'apparaît jamais dans ses propres offres, quelle que soit sa visibilité.
- [ ] Given une carte proposée par B et engagée dans un trade `ONE_ACCEPTED`, When A appelle
      `GET /card/offers`, Then l'offre de B est toujours listée avec `reserved: true`.
- [ ] Given un `CardId` ne correspondant à aucune carte, When A appelle `GET /card/offers`, Then la réponse
      est 404 (comportement inchangé).
- [ ] Given un joueur B en `private`, When A appelle `GET /autocomplete/users` avec un fragment de son nom,
      Then B n'est pas suggéré.
- [ ] Given un joueur B en `trade` sans aucun binder coché, When A appelle `GET /autocomplete/users`, Then B
      n'est pas suggéré.
- [ ] Given un joueur B en `trade` proposant 5 exemplaires au total sur une collection de 40, When A appelle
      `GET /autocomplete/users`, Then B est suggéré avec `card_count: 5`.
- [ ] Given un joueur B en `public` avec 40 exemplaires, When A appelle `GET /autocomplete/users`, Then B est
      suggéré avec `card_count: 40`.
- [ ] Given un trade existant entre A et B, When A appelle `POST /trades/{trade_id}/cards` avec une carte de B
      que B ne propose pas, Then la réponse est 404 et aucune ligne `trade_card` n'est ajoutée.
- [ ] Given B proposant 2 exemplaires d'une carte, When A appelle `POST /trades/{trade_id}/cards` en demandant
      3 exemplaires, Then la réponse est 404 et le trade est inchangé ; When il en demande 2, Then
      l'opération réussit.
- [ ] Given un trade où A met de son côté une carte qu'il ne propose à personne (binder non coché, rareté
      fermée, ou visibilité `private`), When il appelle `POST /trades/{trade_id}/cards`, Then l'opération
      réussit.
- [ ] Given une carte de B ajoutée à un trade, When B cesse ensuite de la proposer et que A appelle
      `POST /trades/{trade_id}/cards/remove`, Then le retrait réussit.
- [ ] Given un trade existant, When A appelle `POST /trades`, Then le comportement est inchangé par cette spec
      (l'endpoint ne transporte aucune carte).
- [ ] Given un utilisateur en `private`, When il appelle `GET /collection`, `GET /collection/stats` et
      `GET /collection/visibility/rarities`, Then il voit sa collection complète, inchangée par cette spec.
- [ ] Given B qui décoche un binder, When A relance immédiatement la même recherche, Then les cartes de ce
      binder ont disparu des résultats sans attendre un import.
- [ ] La sémantique de `quantity` (`/card/offers`), `owner_count` (`/search/card`) et `card_count`
      (`/autocomplete/users`) est documentée dans `doc/openapi.yml`, ainsi que le cas de 404 de
      `POST /trades/{trade_id}/cards`.
- [ ] `mise run checks` passe sans erreur (inclut `rebuild-db-doc`, `sqlx-prepare`, tests et lint).
- [ ] `mise run lint-frontend` passe sans erreur.

## Hors scope

- Modification des écrans frontend : les états vides et les libellés existants suffisent, aucun contrat
  d'API ne change de forme.
- Invalidation rétroactive des trades en cours dont les cartes cessent d'être proposées (un joueur qui décoche
  un binder après avoir accepté un échange) — les transitions de la spec 015 restent inchangées.
- Décompte des exemplaires réservés par un trade dans la quantité proposée : `reserved` reste un drapeau
  informatif, il ne réduit pas `quantity`.
- Prix de vente personnalisé par vendeur : `selling_price` reste le `trend` du price guide (spec 007).
- Endpoint de consultation de la collection d'un tiers : `GET /search/card?player_username=` reste le seul
  moyen, aucune route dédiée n'est créée.
- Règles de mise à l'échange plus fines que « par rareté » (par set, par carte, par prix, par état).
