# Spec : Endpoint de création d'une demande d'échange (`POST /trades`)

## Contexte

`.agents/trade-workflow.instructions.md` décrit le processus complet d'échange carte-contre-carte entre deux
utilisateurs (négociation, acceptation, réservation, confirmation, notation). Aucune brique de ce processus n'existe
encore côté backend : ni table, ni domaine, ni endpoint.

Cette spec couvre uniquement la toute première étape du flux : l'étape 2 « Trade Request » — l'utilisateur _A_
demande la carte d'un utilisateur _B_, sans proposer encore de carte en échange. Les étapes suivantes (contre-offre,
négociation, acceptation, réservation, confirmation, notation, notifications) sont hors scope et feront l'objet de specs
ultérieures.

Le projet dispose déjà de :

- une table `users` (`id`, `username`) alimentée via `POST /user/register` ;
- une table `collection_entry` (ex `card_quantity`) qui trace la possession d'une carte par un utilisateur (`user_id` +
  clé composite `set_code`/`collector_number`/`language_code`/`foil` + `quantity`) ;
- un extracteur `AuthenticatedUser` pour identifier l'utilisateur courant sur les endpoints protégés.

## Objectif

Permettre à un utilisateur authentifié (_A_, l'initiateur) de créer une demande d'échange sur une carte précise possédée
par un autre utilisateur (_B_, le répondant), en statut `PENDING`. Aucune carte de _A_ n'est proposée à ce stade — c'est
uniquement l'expression du souhait « je veux cette carte ».

## Solution

### Nommage

L'entité métier est appelée **trade** (table `trade`, domaine `Trade`, statut `TradeStatus`), pour éviter toute
collision avec le type `Transaction` déjà utilisé par sqlx pour les transactions SQL dans le code Rust.

### Route et accès

- Nouvelle route `POST /trades`, nouveau routeur `create_trade_router` (même pattern que `create_card_router` /
  `create_user_router`), monté dans `infrastructure.rs`.
- Authentification requise (`AuthenticatedUser`) : l'utilisateur du token est l'initiateur _A_. Pas d'endpoint public.

### Identification de la carte demandée

- Le payload identifie la carte via sa clé composite existante : `set_code`, `collector_number`, `language_code`,
  `foil` — cohérent avec la clé primaire de `card`/`collection_entry`.
- Le payload identifie le répondant _B_ via `respondent_user_id`. L'initiateur _A_ n'est pas dans le payload, il est
  déduit du token.
- Le payload inclut une `quantity` (entier ≥ 1) : le nombre d'exemplaires de cette carte demandés à _B_. Sans ce
  champ, impossible de représenter une demande portant sur plusieurs exemplaires identiques (ex. 3 `Sol Ring` non
  foil).

### Modèle de données

Un trade doit pouvoir porter plusieurs cartes de chaque côté (contre-proposition à l'étape 3), pas seulement la
carte unique demandée à la création. Le schéma introduit donc deux tables dès cette brique, même si cet endpoint
n'alimente que le strict nécessaire pour l'étape 2 :

**Table `trade`** :

- un identifiant propre au trade ;
- l'initiateur (`initiator_user_id`, FK `users`) et le répondant (`respondent_user_id`, FK `users`) ;
- un statut ;
- deux colonnes de montant, en centimes, nullable : le montant dû par _A_ à _B_ et le montant dû par _B_ à _A_.
  Elles restent `NULL` tant que le trade n'est pas dans son statut final — elles ne sont figées qu'à l'étape
  8 « Trade Confirmation » (statut → `COMPLETED`), pour conserver une trace immuable du delta même si les prix du
  marché évoluent ensuite. Cet endpoint ne les renseigne pas (création → `PENDING`).
- deux colonnes d'horodatage d'acceptation, nullable : `initiator_accepted_at` et `respondent_accepted_at`. Le
  modèle de données actuel ne permettait pas de savoir _qui_, parmi les deux parties, avait accepté un trade en
  statut `ONE_ACCEPTED` — ces colonnes comblent ce manque, sur le même principe que les deux colonnes de montant dû
  (une colonne par partie plutôt qu'une table de jointure séparée, cohérent avec un modèle de trade à exactement
  deux parties fixes). Un horodatage (plutôt qu'un simple booléen) trace en plus _quand_ l'acceptation a eu lieu.
  `ONE_ACCEPTED` correspond à une seule des deux colonnes renseignée, `FULLY_ACCEPTED` aux deux renseignées. Cet
  endpoint ne renseigne jamais ces colonnes (aucune notion d'acceptation à l'étape « Trade Request » — hors scope,
  voir plus bas), mais les remet à `NULL` toutes les deux lorsqu'il fait repasser un trade `ONE_ACCEPTED` à
  `PENDING` (voir « Fusion dans un trade actif ») : une modification annule toute acceptation en cours, cohérent
  avec `trade-workflow.instructions.md` (étape 5b).
- les horodatages de création/mise à jour.

Le champ statut doit être en mesure de porter l'ensemble des états du cycle de vie décrit dans
`trade-workflow.instructions.md` (`PENDING`, `ONE_ACCEPTED`, `FULLY_ACCEPTED`, `COMPLETED`, `CLOSED`, `ABANDONED`), même
si cet endpoint n'écrit que `PENDING` — pour éviter une migration de schéma dès la prochaine brique
(contre-proposition).

**Table de jointure `trade_card`** (nouvelle, remplace toute référence directe à une carte sur `trade`) :

- FK vers `trade` ;
- la référence de la carte (clé composite, FK `card`) ;
- le propriétaire de cette carte dans ce trade (`owner_user_id`, FK `users`) ;
- une `quantity` (nombre d'exemplaires de cette carte inclus dans le trade, pour ce propriétaire).

Pas de colonne « côté » : elle serait redondante avec `owner_user_id`, qui, comparé à `trade.initiator_user_id` /
`trade.respondent_user_id`, permet de déduire si la carte est offerte par l'initiateur ou par le répondant.
`owner_user_id` doit être cohérent avec le trade parent (égal à `trade.respondent_user_id` ou à
`trade.initiator_user_id`) — vérifié à l'insertion, pas contraint nativement en base (pas de contrainte multi-tables
portable en SQL standard).

Cet endpoint insère exactement une ligne dans `trade_card`, pour la carte demandée de _B_, avec `owner_user_id` égal
à _B_ et `quantity` égale à celle fournie dans le payload. Aucune ligne pour une carte de _A_ n'est créée (cohérent
avec « Aucune carte de _A_ n'est proposée à ce stade » dans `trade-workflow.instructions.md`).

Architecture hexagonale, même découpage que l'existant : handler (`infrastructure/adapter_in/trade`) → use case trait →
service (`application/service`) → repository port (domaine) → adapter (`infrastructure/adapter_out/repository`).

### Validations métier

À la création, dans cet ordre :

1. _B_ doit posséder au moins la quantité demandée de la carte : une ligne `collection_entry` existe pour
   `respondent_user_id` + la clé composite fournie, avec `quantity >= quantity` (celle du payload). À défaut (carte
   inexistante, _B_ n'en possède aucune, ou en possède moins que demandé), → 404. Cette vérification porte
   uniquement sur la `quantity` du payload courant, indépendamment de ce qui est déjà engagé dans un trade actif
   (pas de comptabilité de réservation globale à ce stade — cohérent avec `trade-workflow.instructions.md`, où la
   réservation n'intervient qu'à l'étape `ONE_ACCEPTED`).
2. _A_ ne peut pas se cibler lui-même : `respondent_user_id` doit être différent de l'id de l'initiateur (issu du
   token). Sinon → 400.
3. Réutilisation d'un trade actif plutôt que blocage : voir « Fusion dans un trade actif » ci-dessous.

### Fusion dans un trade actif

Un trade actif est un trade dont le statut est `PENDING`, `ONE_ACCEPTED` ou `FULLY_ACCEPTED` (à l'exclusion de
`COMPLETED`, `CLOSED`, `ABANDONED`, qui sont terminaux). La recherche d'un trade actif entre _A_ et _B_ est
**indépendante du sens** initiateur/répondant : un trade actif `(initiator=A, respondent=B)` ou
`(initiator=B, respondent=A)` compte comme la même négociation en cours entre ces deux utilisateurs — cohérent avec
le modèle de contre-proposition de `trade-workflow.instructions.md`, où chaque partie peut ajouter des cartes au même
trade indépendamment de qui l'a initié. S'il existe plusieurs trades actifs pour la paire (cas résiduel), le plus
ancien (`created_at`) est utilisé.

Comportement selon le résultat de cette recherche :

- **Aucun trade actif** : un nouveau trade est créé en `PENDING`, comme décrit dans « Modèle de données » — comportement
  inchangé.
- **Trade actif en `PENDING`** : la carte demandée est ajoutée au trade existant (nouvelle ligne `trade_card`,
  `owner_user_id` = _B_, `quantity` = celle du payload). Le statut reste `PENDING`. Si la carte (même clé composite,
  même `owner_user_id`) est déjà présente dans ce trade, sa `quantity` est **incrémentée** de celle du payload plutôt
  que rejetée. Dans tous les cas, `trade.updated_at` est mis à jour (une modification du trade a eu lieu, cohérent
  avec la table « Modification Rules » de `trade-workflow.instructions.md`).
- **Trade actif en `ONE_ACCEPTED`** : même comportement que `PENDING` (ajout ou incrément de `trade_card`), mais le
  statut du trade **repasse à `PENDING`** et `initiator_accepted_at`/`respondent_accepted_at` sont remises à `NULL`
  toutes les deux — cohérent avec la règle générale de `trade-workflow.instructions.md` : toute modification pendant
  `ONE_ACCEPTED` annule l'acceptation en cours et remet le trade en négociation.
- **Trade actif en `FULLY_ACCEPTED`** : la modification est refusée → 409. Cohérent avec `trade-workflow.instructions.md`
  (« Modification Rules » : aucune modification possible à partir de `FULLY_ACCEPTED`).

Cette fusion remplace l'ancienne règle de doublon strict par triplet (initiateur, répondant, carte) : la portée du
« trade actif » devient la paire d'utilisateurs (_A_, _B_), pas un triplet incluant la carte — une seule négociation
active à la fois entre deux utilisateurs, sur laquelle toutes les cartes demandées viennent s'agréger.

### Réponse

Réponse minimale : `201 Created`, sans body de ressource complète (l'identifiant du trade créé peut être exposé via un
header `Location`, pas nécessairement dans le body). Aucun GET de consultation n'est requis par cette spec.

### OpenAPI

Documenter `POST /trades` dans `doc/openapi.yml` (payload, réponses 201/400/401/404/409).

## Cas d'erreurs

- Token bearer manquant ou invalide → 401.
- `respondent_user_id` égal à l'id de l'initiateur → 400.
- Carte (clé composite) inexistante en base, ou existante mais non possédée par _B_ en quantité suffisante (pas de
  ligne `collection_entry`, ou `quantity` insuffisante face à celle demandée) → 404.
- `respondent_user_id` ne correspondant à aucun utilisateur connu → 404 (même traitement que « B ne possède pas
  assez de la carte », pas de distinction nécessaire côté API).
- Un trade actif (`PENDING`, `ONE_ACCEPTED` ou `FULLY_ACCEPTED`, dans les deux sens) existe déjà entre _A_ et _B_ et
  est en `FULLY_ACCEPTED` → 409 (aucune modification possible sur un trade déjà pleinement accepté). Dans les deux
  autres statuts actifs (`PENDING`, `ONE_ACCEPTED`), pas d'erreur : voir « Fusion dans un trade actif ».
- `quantity` absente, non entière, nulle ou négative dans le payload → 400.
- Payload incomplet ou mal formé (champs manquants, `foil` non booléen, etc.) → 400.

## Hors scope

- Contre-proposition, négociation, acceptation, réservation de cartes, confirmation d'échange physique, notation — tout
  ce qui suit l'étape « Trade Request » dans `trade-workflow.instructions.md`.
- Notifications (aucun système de notification n'existe actuellement dans le projet).
- Endpoint de consultation (GET) d'un ou plusieurs trades.
- Interface frontend.

## Critères d'acceptance

- [ ] `POST /trades` avec un payload valide (carte possédée par _B_, `respondent_user_id` différent de l'initiateur,
      aucun trade actif entre _A_ et _B_) crée un nouveau trade en statut `PENDING` et répond `201`.
- [ ] Le trade créé référence bien l'initiateur (déduit du token), le répondant fourni, et la carte demandée.
- [ ] Requête sans token d'authentification → `401`.
- [ ] `respondent_user_id` égal à l'id de l'initiateur → `400`, aucun trade créé.
- [ ] Clé composite de carte ne correspondant à aucune carte en base → `404`, aucun trade créé.
- [ ] Carte existante mais absente de la collection de _B_ (ou `quantity = 0`) → `404`, aucun trade créé.
- [ ] Carte possédée par _B_ mais en quantité insuffisante (ex. _B_ en a 2, `quantity` demandée = 3) → `404`, aucun
      trade créé.
- [ ] `respondent_user_id` ne correspondant à aucun utilisateur enregistré → `404`, aucun trade créé.
- [ ] `quantity` absente, nulle ou négative dans le payload → `400`, aucun trade créé.
- [ ] Une nouvelle demande de carte alors qu'un trade `PENDING` existe déjà entre _A_ et _B_ → `201`, aucun nouveau
      trade créé, une ligne `trade_card` est ajoutée au trade existant, le statut reste `PENDING`.
- [ ] Une nouvelle demande de carte alors qu'un trade `ONE_ACCEPTED` existe déjà entre _A_ et _B_ → `201`, ligne
      `trade_card` ajoutée, le statut du trade repasse à `PENDING`, et `initiator_accepted_at`/`respondent_accepted_at`
      sont remises à `NULL` toutes les deux.
- [ ] Une nouvelle demande de carte alors qu'un trade `FULLY_ACCEPTED` existe déjà entre _A_ et _B_ → `409`, aucune
      modification du trade ni de ses `trade_card`.
- [ ] Une nouvelle demande sur une carte déjà présente dans le trade actif (même clé composite, même `owner_user_id`)
      → `201`, la `quantity` de la ligne `trade_card` existante est incrémentée de celle du payload (pas de nouvelle
      ligne, pas d'erreur).
- [ ] Le trade actif est retrouvé quel que soit le sens initiateur/répondant : si _B_ avait initié un trade actif
      vers _A_ (`initiator=B`, `respondent=A`), une nouvelle demande de _A_ vers _B_ fusionne dans ce trade plutôt que
      d'en créer un nouveau.
- [ ] Une nouvelle demande entre _A_ et _B_ alors qu'un trade antérieur entre eux est `COMPLETED`, `CLOSED` ou
      `ABANDONED` n'est pas bloquée ni fusionnée : un nouveau trade est créé.
- [ ] Une nouvelle demande sur la même carte mais avec un répondant ou un initiateur différent n'est pas affectée par
      un trade actif existant pour une autre paire d'utilisateurs.
- [ ] Payload avec un champ obligatoire manquant ou mal typé → `400`.
- [ ] L'endpoint est documenté dans `doc/openapi.yml`.
- [ ] La table `trade` et son statut couvrent l'ensemble des valeurs du cycle de vie défini dans
      `trade-workflow.instructions.md` ; cet endpoint n'écrit jamais explicitement `ONE_ACCEPTED`/`FULLY_ACCEPTED`/etc.,
      mais peut faire repasser un trade `ONE_ACCEPTED` à `PENDING`.
- [ ] La création (nouveau trade) insère une ligne `trade_card` référençant le trade créé, la carte demandée,
      `owner_user_id` égal au répondant _B_ et `quantity` égale à celle du payload ; aucune ligne pour une carte de
      l'initiateur n'est créée.
- [ ] Les deux colonnes de montant dû sur `trade` restent `NULL` après la création ou la fusion.
- [ ] Les deux colonnes `initiator_accepted_at`/`respondent_accepted_at` restent `NULL` après une création ou une
      fusion dans un trade `PENDING` ; elles sont explicitement remises à `NULL` (si elles ne l'étaient pas déjà) lors
      d'une fusion dans un trade `ONE_ACCEPTED`.
