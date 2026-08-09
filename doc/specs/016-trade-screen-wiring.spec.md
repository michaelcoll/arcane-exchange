hvfiesd# Spec : Branchement de l'écran de trade sur le backend

## Contexte

L'écran `frontend-vue/app/pages/trade/index.vue` reproduit la maquette du cycle de vie d'un échange (colonnes
donne/reçois, balance, stepper, bannières contextuelles, modales de confirmation) mais fonctionne **intégralement sur
des données mockées** : partenaire en dur, cartes en dur, et une machine à états recalculée côté client à partir de
booléens locaux (`acceptedMe`, `confirmedThem`, …).

Côté backend, la spec 005 a livré `POST /trades` et la spec 015 les quatre transitions (`accept` / `abandon` /
`confirm` / `rate`). Il manque tout ce qui permettrait de brancher l'écran :

- Aucun endpoint de **lecture** : ni détail d'un trade, ni liste de mes trades. `find_by_id` et `find_trade_cards`
  existent au niveau repository mais ne sont exposés nulle part.
- `POST /trades` renvoie `201` **sans body** : le front ne récupère pas l'id du trade qu'il vient de créer, donc ne peut
  pas rediriger vers l'écran (qui est d'ailleurs sur une route sans paramètre, `/trade`).
- Aucun moyen de **retirer** une carte d'un trade (`quantity: 0` est explicitement rejeté, et la fusion ne sait
  qu'incrémenter), ni d' **ajouter une de ses propres cartes** : `POST /trades` valide toujours la possession chez le
  `respondent_user_id` du payload et insère `owner_user_id = respondent`. Seules les cartes de l'autre partie peuvent
  donc être posées, ce qui rend la contre-proposition du workflow inapplicable telle quelle depuis l'écran.

## Objectif

Rendre l'écran de trade réellement fonctionnel de bout en bout : ouvrir un trade existant, voir son contenu et son
statut réels, modifier son contenu (ajouter/retirer des cartes des deux côtés) et déclencher les quatre transitions, le
tout sur des données backend.

## Solution

### Découplage de la création d'un trade

`POST /trades` est reconstruit pour ne créer qu'une **coquille de trade**, sans carte :

- Payload réduit à l'identification du partenaire, désigné par son **username** — jamais par l'identifiant interne
  d'utilisateur, qui n'est exposé nulle part ailleurs dans l'API (`CardOffer.owner_username`, `player_username` de
  `GET /search/card`, `UserSuggestion.username`). C'est un changement incompatible du `CreateTradeRequest` actuel.
- Réponse : l' **identifiant du trade**, pour permettre la redirection du front vers l'écran.
- L'invariant « un seul trade actif par paire d'utilisateurs », sur lequel repose `find_active_trade`, est conservé :
  si un trade actif (`PENDING`, `ONE_ACCEPTED` ou `FULLY_ACCEPTED`) existe déjà avec ce joueur, l'endpoint renvoie
  l'identifiant de ce trade existant au lieu d'en créer un second.
- Le trade est créé en `PENDING`, sans aucune carte. **Un trade sans carte devient donc un état valide** — ce n'était
  pas possible auparavant.

L'ajout des cartes se fait ensuite exclusivement par la sous-ressource ci-dessous, y compris pour la toute première
carte demandée depuis la recherche.

### Contenu d'un trade

Deux endpoints sur la sous-ressource cartes du trade :

- **Ajout** : identité complète de la carte (set / numéro de collecteur / langue / foil), propriétaire de la carte
  (l'appelant ou le partenaire) et quantité (au moins 1). La possession est validée chez le propriétaire désigné, comme
  le fait déjà `create_trade`. Si la carte est déjà présente pour ce propriétaire, la quantité est incrémentée
  (comportement actuel de `merge_card_into_trade`).
- **Retrait** : la carte est identifiée de la même façon (identité + propriétaire) et la ligne est retirée
  **entièrement**, quelle que soit sa quantité. Pas de retrait partiel.

Règles communes aux deux, reprises du comportement existant de la fusion :

- Réservés aux deux parties du trade.
- Autorisés en `PENDING` et en `ONE_ACCEPTED` ; refusés à partir de `FULLY_ACCEPTED`.
- Sur un trade `ONE_ACCEPTED`, toute modification le ramène en `PENDING` et annule les deux acceptations, exactement
  comme la fusion aujourd'hui (`merge_card_into_trade`, paramètre `reopen_to_pending`).
- Retirer la dernière carte est autorisé : le trade redevient vide, et reste `PENDING`.

Conséquence sur `accept` (spec 015) : un trade **sans aucune carte n'est pas acceptable** → l'acceptation est refusée
tant que le trade est vide.

### Consultation d'un trade

Un endpoint de détail, réservé aux deux parties, renvoyant tout ce dont l'écran a besoin :

- Le **statut** du trade — c'est désormais la seule source de vérité ; le front ne le recalcule plus.
- Le **partenaire**, par son username.
- Les **cartes des deux côtés**, chacune avec son identité, son nom, sa quantité, son propriétaire, ses prix (même
  structure de prix que les autres listings de cartes) et les identifiants d'image déjà utilisés ailleurs
  (`scryfall_id`, `the_gatherer_id`).
- L' **état de chaque partie**, exprimé du point de vue de l'appelant (moi / le partenaire) pour l'acceptation, la
  confirmation de l'échange physique et la notation. Les colonnes `initiator_*` / `respondent_*` ne sont pas exposées
  telles quelles : la bascule est faite côté backend, comme le fait déjà `resolve_party`.

L'état « réservé » des cartes n'est **pas** un champ par carte : il se déduit du statut du trade (`ONE_ACCEPTED` ou
`FULLY_ACCEPTED`), conformément à la règle posée par la spec 015.

### Liste de mes trades

Un endpoint listant **tous** les trades dont l'appelant est partie, actifs comme terminés :

- Résumé par trade : identifiant, statut, username du partenaire, nombre de cartes de chaque côté, date de dernière mise
  à jour.
- Trié par date de mise à jour décroissante.
- Paginé selon la convention des autres listings du projet (`page` / `page_size`).
- Filtre optionnel sur le statut, répétable pour plusieurs valeurs (même convention que le paramètre `rarity` de
  `GET /search/card`).

Aucun écran de liste n'est demandé à ce stade : l'endpoint est livré seul.

### Frontend

**Routage** — la page devient `/trade/[id]`, avec le middleware `auth` déjà en place. Le bouton « Composer l'échange »
de l'écran de recherche crée le trade avec le propriétaire visé, puis ajoute la carte demandée et redirige vers l'écran.

**Données** — tout le state mocké de `index.vue` disparaît (partenaire, cartes, booléens d'acceptation/confirmation, et
surtout le `computed` qui dérive le statut). Statut, cartes et flags viennent du détail du trade. Après chaque action
(accepter, abandonner, confirmer, noter, ajouter, retirer), l'écran se resynchronise sur le backend plutôt que de mettre
à jour son état local. Les helpers d'affichage de `app/utils/trade.ts` (libellés, tons, stepper,
`isTradeEditable`, `isTradeReserved`) sont conservés tels quels. Un composable de service dédié est ajouté, sur le
modèle de `useCollectionService` / `useSearchService`.

**Sélection des cartes à ajouter** — les boutons « Ajouter une de mes cartes » et « Chercher dans sa collection »
réutilisent les écrans existants en mode sélection : l'écran de collection pour mes cartes, l'écran de recherche filtré
sur le username du partenaire (paramètre `player_username`, spec 013) pour les siennes. Un clic sur une carte l'ajoute
au trade et ramène à l'écran de trade. Le trade cible et le côté visé sont transmis par la navigation.

**Quantités** — chaque ligne affiche `×N` lorsque la quantité est supérieure à 1. Le bouton de retrait supprime la ligne
entière. L'ajout se fait toujours à un exemplaire ; ajouter deux fois la même carte incrémente la quantité.

**Alignements sur le backend** — l'écran présente aujourd'hui des fonctions que le backend ne porte pas ; elles sont
retirées plutôt que simulées :

- Le bouton « Passer la notation » et la notation d'une partie ayant abandonné le trade disparaissent : le backend n'a
  pas de mécanisme de « passer », et refuse toute notation hors statut `COMPLETED` (écart assumé de la spec 015).
- Le sélecteur de mode de comparaison `Prix € / EDHREC %` et la bannière explicative associée sont retirés : aucune
  donnée EDHREC par carte n'est exposée par l'API. L'écran ne compare que les prix. La balance et le delta restent
  calculés côté front à partir des prix des cartes ; les colonnes `initiator_amount_due` / `respondent_amount_due`
  restent inutilisées.
- La note globale du partenaire affichée sous son nom est retirée : aucune note d'utilisateur n'est calculée
  (explicitement reporté par la spec 015).

**Badge « réservée »** — sur les deux colonnes, dès que le statut vaut `ONE_ACCEPTED` ou `FULLY_ACCEPTED`, chaque carte
porte le badge cadenas et le bouton de retrait est remplacé par une icône figée, conformément à la règle déjà documentée
par la spec 015.

### OpenAPI et collection Bruno

Les nouvelles routes et la nouvelle forme de `POST /trades` sont documentées dans `doc/openapi.yml` (généré via
`utoipa`), et la collection Bruno `collection/trades` est mise à jour en conséquence.

## Cas d'erreurs

- Token bearer manquant ou invalide → `401` sur tous les endpoints.
- Trade inconnu → `404` ; appelant qui n'est ni l'initiateur ni le répondant → `403` (consultation, ajout, retrait,
  comme pour les quatre transitions).
- Création d'un trade avec un username inconnu → `404`.
- Création d'un trade avec son propre username → `400` (comportement `SelfTrade` actuel).
- Ajout d'une carte inconnue, ou non possédée en quantité suffisante par le propriétaire désigné → `404`.
- Ajout avec une quantité nulle ou négative → `400`.
- Ajout ou retrait sur un trade `FULLY_ACCEPTED`, `COMPLETED`, `CLOSED` ou `ABANDONED` → `409`.
- Retrait d'une carte absente du trade → `404`.
- Acceptation d'un trade ne contenant aucune carte → `409`.
- Filtre de statut invalide sur la liste → `400`.
- Échec d'une action côté écran : le message d'erreur est affiché et l'écran se resynchronise sur l'état réel du
  backend, sans appliquer la modification localement. En particulier, un trade abandonné en cascade par une acceptation
  concurrente (spec 015) doit se refléter au premier rechargement.
- Trade inaccessible ou inexistant à l'ouverture de `/trade/[id]` : l'écran affiche une erreur explicite plutôt qu'un
  écran vide.

## Hors scope

- Écran de liste des trades (l'endpoint est livré, l'écran non).
- Notifications sur les transitions (aucun système de notification dans le projet).
- Mécanisme de « passer » la notation, notation après abandon, note globale d'un utilisateur.
- Exposition du `% EDHREC` par carte.
- Delta cash persisté (`initiator_amount_due` / `respondent_amount_due`) : reste calculé à l'affichage, informatif.
- Retrait partiel d'une quantité et réglage fin des quantités depuis l'écran.
- Temps réel / rafraîchissement automatique : la synchronisation se fait sur action et au chargement.

## Critères d'acceptance

### Création découplée

- [ ] `POST /trades` avec le username d'un autre joueur, sans aucune carte → trade créé en `PENDING`, sans carte, et la
      réponse contient son identifiant.
- [ ] `POST /trades` alors qu'un trade actif existe déjà avec ce joueur → aucun second trade créé, la réponse renvoie
      l'identifiant du trade actif existant.
- [ ] `POST /trades` avec un username inconnu → `404` ; avec son propre username → `400`.
- [ ] Un trade sans carte est refusé à l'acceptation → `409`.

### Contenu

- [ ] Ajout d'une carte du partenaire à un trade `PENDING` → la carte apparaît côté partenaire, statut inchangé.
- [ ] Ajout d'une de mes propres cartes à un trade `PENDING` → la carte apparaît de mon côté, statut inchangé.
- [ ] Ajout d'une carte non possédée en quantité suffisante par le propriétaire désigné → `404`, trade inchangé.
- [ ] Ajout deux fois de la même carte pour le même propriétaire → une seule ligne, quantité cumulée.
- [ ] Ajout ou retrait sur un trade `ONE_ACCEPTED` → statut ramené à `PENDING`, les deux acceptations annulées.
- [ ] Ajout ou retrait sur un trade `FULLY_ACCEPTED`, `COMPLETED`, `CLOSED` ou `ABANDONED` → `409`, trade inchangé.
- [ ] Retrait d'une carte de quantité 3 → la ligne disparaît entièrement.
- [ ] Retrait d'une carte absente du trade → `404`.
- [ ] Retrait de la dernière carte → trade vide, toujours `PENDING`.
- [ ] Ajout ou retrait par un utilisateur qui n'est pas partie au trade → `403`.

### Consultation

- [ ] La consultation d'un trade renvoie son statut, le username du partenaire, et les cartes des deux côtés avec nom,
      quantité, propriétaire, prix et identifiants d'image.
- [ ] Les états d'acceptation, de confirmation et de notation sont renvoyés du point de vue de l'appelant :
      l'initiateur et le répondant du même trade voient leurs propres flags sur le champ « moi ».
- [ ] Consultation par un utilisateur qui n'est pas partie au trade → `403` ; trade inconnu → `404`.

### Liste

- [ ] La liste renvoie tous les trades de l'appelant, actifs et terminés, triés par date de mise à jour décroissante,
      avec le username du partenaire et le nombre de cartes de chaque côté.
- [ ] Un trade auquel l'appelant n'est pas partie n'apparaît jamais dans sa liste.
- [ ] Le filtre de statut restreint la liste aux statuts demandés, et accepte plusieurs valeurs.
- [ ] La pagination suit la convention `page` / `page_size` des autres listings.

### Écran

- [ ] `/trade/[id]` affiche le statut, le partenaire et les cartes issus du backend ; aucune donnée en dur ne subsiste
      dans la page.
- [ ] Le statut affiché provient du backend : le `computed` local qui le dérivait a disparu.
- [ ] « Composer l'échange » depuis la recherche crée le trade, y ajoute la carte visée, et redirige vers
      `/trade/[id]`.
- [ ] Accepter / abandonner / confirmer / noter appellent l'endpoint correspondant, puis l'écran reflète le statut
      renvoyé par le backend (stepper, bannière, pastille, actions disponibles).
- [ ] Une action refusée par le backend affiche une erreur et laisse l'écran sur l'état réel du trade.
- [ ] « Ajouter une de mes cartes » ouvre l'écran de collection en mode sélection ; choisir une carte l'ajoute côté « Je
      donne » et ramène au trade.
- [ ] « Chercher dans sa collection » ouvre l'écran de recherche filtré sur le partenaire ; choisir une carte l'ajoute
      côté « Je reçois » et ramène au trade.
- [ ] Le bouton de retrait d'une carte retire la ligne entière, et la modale d'avertissement est présentée quand le
      trade est `ONE_ACCEPTED`.
- [ ] Une carte de quantité supérieure à 1 affiche `×N`.
- [ ] En `ONE_ACCEPTED` et `FULLY_ACCEPTED`, toutes les cartes des deux colonnes portent le badge « réservée » et ne
      sont plus retirables depuis l'écran.
- [ ] Le sélecteur `Prix € / EDHREC %`, sa bannière explicative, la note globale du partenaire, le bouton « Passer la
      notation » et la notation après abandon ne sont plus présents dans l'écran.
- [ ] Ouvrir `/trade/[id]` sur un trade inexistant ou inaccessible affiche une erreur explicite.

### Documentation

- [ ] Les nouvelles routes et la nouvelle forme de `POST /trades` sont documentées dans `doc/openapi.yml`.
- [ ] La collection Bruno `collection/trades` couvre la création, la consultation, la liste, l'ajout et le retrait d'une
      carte.
