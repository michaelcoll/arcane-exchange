# Spec : Transitions de statut d'un trade (accepter, abandonner, confirmer, noter)

## Contexte

`.agents/trade-workflow.instructions.md` décrit le cycle de vie complet d'un trade :

```
PENDING → ONE_ACCEPTED → FULLY_ACCEPTED → COMPLETED → CLOSED
ABANDONED ← reachable from any status before COMPLETED
```

`005-trade-request-endpoint.spec.md` couvre uniquement la création et la fusion dans un trade actif (statut
`PENDING`, et le retour `ONE_ACCEPTED` → `PENDING` en cas de modification). Aucune des transitions déclenchées par une
action explicite d'un utilisateur (accepter, abandonner, confirmer l'échange physique, noter) n'existe encore côté
backend.

Cette spec couvre ces quatre actions et les transitions de statut qu'elles déclenchent.

## Objectif

Permettre à chacune des deux parties d'un trade (initiateur ou répondant, identifié via le token d'authentification)
de :

- **Accepter** le trade (`PENDING`/`ONE_ACCEPTED` → `ONE_ACCEPTED`/`FULLY_ACCEPTED`).
- **Abandonner** le trade, à tout moment avant `COMPLETED` (→ `ABANDONED`).
- **Confirmer** l'échange physique une fois `FULLY_ACCEPTED` (→ `COMPLETED` quand les deux parties ont confirmé).
- **Noter** l'autre partie une fois `COMPLETED` (→ `CLOSED` quand les deux parties ont noté).

Dans tous les cas, la partie qui agit est déduite du token — jamais d'un champ du payload — en la comparant à
`trade.initiator_user_id` / `trade.respondent_user_id`.

La maquette (`maquette/screen_collection.jsx`, `maquette/components.jsx`) affiche déjà l'état « réservée » d'une carte
dans la grille de collection (badge cadenas + liseré violet), avec exactement la même règle de dérivation que celle
retenue plus bas (`reserved` vrai si la carte appartient à un trade `ONE_ACCEPTED` ou `FULLY_ACCEPTED`). Cette spec
inclut donc l'exposition de cette information par `GET /collection`, pour permettre au frontend de reproduire cet
affichage.

## Solution

### Endpoints

Un endpoint dédié par action, cohérent avec le style déjà en place dans le projet (verbes en kebab-case pour les
actions ponctuelles) et avec `POST /trades` (pas de PUT/PATCH introduit) :

- `POST /trades/{trade_id}/accept`
- `POST /trades/{trade_id}/abandon`
- `POST /trades/{trade_id}/confirm`
- `POST /trades/{trade_id}/rate`

Les quatre nécessitent `AuthenticatedUser`. Aucun payload requis sauf pour `rate` (voir plus bas). Réponse `204 No
Content` en cas de succès pour les quatre (pas de body, cohérent avec le style « réponse minimale » de la spec 005).

### Autorisation commune

Pour les quatre endpoints : l'utilisateur du token doit être l'initiateur ou le répondant du trade ciblé. Sinon →
`403`. Trade `{trade_id}` inconnu → `404`.

### Accepter (`POST /trades/{trade_id}/accept`)

- Autorisé uniquement si le trade est en `PENDING` ou `ONE_ACCEPTED`. Sinon → `409`.
- Détermine si l'appelant est l'initiateur ou le répondant, et renseigne `initiator_accepted_at` ou
  `respondent_accepted_at` (celle qui correspond) avec l'horodatage courant.
- Si l'appelant a déjà accepté ce trade (sa colonne d'acceptation est déjà renseignée) → `409` (pas de double
  acceptation).
- Recalcule le statut : si les deux colonnes d'acceptation sont renseignées → `FULLY_ACCEPTED` ; sinon →
  `ONE_ACCEPTED`.
- **Réservation et abandon en cascade** (uniquement lors du premier passage `PENDING` → `ONE_ACCEPTED`, pas lors du
  second passage `ONE_ACCEPTED` → `FULLY_ACCEPTED`) : la réservation d'une carte est un état dérivé — une carte est
  considérée réservée si elle apparaît dans un `trade_card` d'un trade en statut `ONE_ACCEPTED` ou `FULLY_ACCEPTED` ;
  aucune colonne dédiée n'est ajoutée. Au moment où ce trade passe à `ONE_ACCEPTED`, tout autre trade actif
  (`PENDING` ou `ONE_ACCEPTED`, hors ce trade) partageant au moins une carte (même clé composite carte + même
  `owner_user_id`, via `trade_card`) avec ce trade est automatiquement mis à `ABANDONED`.

### Abandonner (`POST /trades/{trade_id}/abandon`)

- Autorisé si le trade n'est pas déjà `COMPLETED`, `CLOSED` ou `ABANDONED`. Sinon → `409`.
- Passe le statut à `ABANDONED`, quelle que soit la partie qui appelle (pas d'accord mutuel requis, cohérent avec
  `trade-workflow.instructions.md`).
- Aucune action supplémentaire requise sur les cartes réservées : la réservation étant un état dérivé du statut, le
  passage à `ABANDONED` les libère de fait (elles ne sont plus comptées comme réservées par ce trade).

### Confirmer l'échange physique (`POST /trades/{trade_id}/confirm`)

- Autorisé uniquement si le trade est `FULLY_ACCEPTED`. Sinon → `409`.
- Introduit deux nouvelles colonnes sur `trade`, sur le même principe que `initiator_accepted_at` /
  `respondent_accepted_at` : `initiator_confirmed_at` / `respondent_confirmed_at` (nullable), pour distinguer qui a
  confirmé l'échange physique.
- Détermine si l'appelant est l'initiateur ou le répondant, et renseigne la colonne de confirmation correspondante.
- Si l'appelant a déjà confirmé → `409` (pas de double confirmation).
- Quand les deux colonnes sont renseignées → statut `COMPLETED`. Tant qu'une seule est renseignée, le statut reste
  `FULLY_ACCEPTED`.

### Noter (`POST /trades/{trade_id}/rate`)

- Autorisé uniquement si le trade est `COMPLETED`. Sinon → `409`.
- Payload : une note entière `rating`, de 0 à 5 inclus (`trade-workflow.instructions.md` : « zero to five stars »).
- Introduit deux nouvelles colonnes sur `trade`, sur le même principe : `initiator_rating` / `respondent_rating`
  (nullable), pour la note donnée par chaque partie à l'autre.
- Détermine si l'appelant est l'initiateur ou le répondant, et renseigne la colonne de note correspondante avec la
  valeur du payload.
- Si l'appelant a déjà noté ce trade → `409` (une note n'est pas modifiable).
- Quand les deux colonnes sont renseignées → statut `CLOSED`, et la note finale du trade est la moyenne des deux
  notes.
- **Écart volontaire avec `trade-workflow.instructions.md`** : le workflow prévoit qu'une partie puisse explicitement
  « passer » sa notation sans bloquer la clôture (note finale = celle de l'unique partie ayant noté). Cette spec
  n'introduit pas de mécanisme de « passer » : le trade ne passe en `CLOSED` que lorsque les **deux** parties ont
  noté. Si une seule des deux note, le trade reste `COMPLETED` indéfiniment (cohérent avec l'absence d'expiration de
  trade, déjà hors scope MVP). Le mécanisme de « passer » et le calcul de la note globale d'un utilisateur
  (moyenne sur l'ensemble de ses trades) sont laissés à une spec ultérieure.

### Exposition de l'état de réservation (`GET /collection`)

La maquette affiche, dans la grille de collection, un badge « Réservée » (icône cadenas) sur les cartes engagées dans
un trade verrouillé — cohérent avec la règle de réservation dérivée définie plus haut. Pour reproduire cet
affichage côté `frontend-vue`, `GET /collection` doit exposer cette information :

- Ajout d'un champ `reserved: bool` sur `CollectionCardResponse` (`src/ccpt/infrastructure/adapter_in/collection/dto.rs`).
- Renseigné uniquement en mode « ma collection » (quand `collection_entry` est présent, c'est-à-dire pour
  l'utilisateur authentifié consultant ses propres cartes) : `true` si au moins une ligne `trade_card` référence
  cette carte avec `owner_user_id` égal à l'utilisateur courant, dans un trade `ONE_ACCEPTED` ou `FULLY_ACCEPTED` ;
  `false` sinon.
- Absent (ou `false`, sans distinction côté client) en mode recherche publique (`owner_count` présent,
  `collection_entry` absent) : cette spec ne couvre pas l'affichage de la réservation sur la collection d'un autre
  utilisateur.
- Champ documenté dans `doc/openapi.yml` (généré automatiquement via `utoipa`, comme le reste du schéma).

### Frontend

- **Grille de collection** : reproduire le badge « Réservée » (icône cadenas, liseré violet sur la carte) de la
  maquette (`maquette/components.jsx`, composant `CardCell`, classes `.reserved-flag` / `.card-cell.is-reserved`
  dans `maquette/styles.css`) sur les cartes où `reserved = true`, en traduisant en Vue/Tailwind selon
  `.agents/design-system.instructions.md` — pas de copie directe du CSS/JSX de la maquette.
- **Écran de trade** : la maquette (`maquette/screen_trade.jsx`, composant `TradeColumn`) affiche le même badge
  cadenas sur chaque carte des deux colonnes (donne/reçois) et remplace le bouton de retrait par une icône cadenas
  figée, dès que `trade.status` vaut `ONE_ACCEPTED` ou `FULLY_ACCEPTED` (pas de champ `reserved` par carte côté
  trade : la règle se déduit directement du statut du trade, déjà porté par cette spec). Cet écran dépend d'un
  endpoint de consultation du détail d'un trade (avec ses cartes) qui n'existe pas encore et n'est pas couvert par
  cette spec — seule la règle de dérivation est documentée ici, pour qu'elle soit reprise telle quelle par la future
  spec de consultation des trades.

### OpenAPI

Documenter les quatre nouvelles routes dans `doc/openapi.yml`, même style que l'entrée `/trades` existante (tags
`trades`, `security: bearer_auth`, réponses `204/403/404/409`).

## Cas d'erreurs

- Token bearer manquant ou invalide → `401`, sur les quatre endpoints.
- `{trade_id}` ne correspondant à aucun trade → `404`, sur les quatre endpoints.
- L'appelant n'est ni l'initiateur ni le répondant du trade → `403`, sur les quatre endpoints.
- `accept` sur un trade `FULLY_ACCEPTED`, `COMPLETED`, `CLOSED` ou `ABANDONED` → `409`.
- `accept` par une partie ayant déjà accepté ce trade → `409`.
- `abandon` sur un trade déjà `COMPLETED`, `CLOSED` ou `ABANDONED` → `409`.
- `confirm` sur un trade qui n'est pas `FULLY_ACCEPTED` → `409`.
- `confirm` par une partie ayant déjà confirmé → `409`.
- `rate` sur un trade qui n'est pas `COMPLETED` → `409`.
- `rate` par une partie ayant déjà noté ce trade → `409`.
- `rate` avec une note absente, non entière, ou hors de l'intervalle 0-5 → `400`.

## Hors scope

- Notifications pour chaque transition (aucun système de notification n'existe actuellement dans le projet, cohérent
  avec la spec 005).
- Mécanisme explicite de « passer » la notation, et calcul de la note globale d'un utilisateur (moyenne sur
  l'ensemble de ses trades) — voir « Écart volontaire » ci-dessus.
- Notation d'une partie ayant abandonné un trade (mentionnée dans `trade-workflow.instructions.md`, mais ne
  déclenche aucune transition de statut : traitée dans une spec ultérieure si besoin).
- Affichage du badge « carte réservée » sur l'écran de trade (dépend d'un endpoint de consultation du détail d'un
  trade qui n'existe pas encore — voir « Frontend » ci-dessus).
- Affichage du badge « carte réservée » sur la collection d'un autre utilisateur en mode recherche publique.
- Modification du contenu d'un trade (ajout/retrait de cartes) autre que celle déjà couverte par la fusion dans un
  trade actif (spec 005).
- Interface frontend pour les actions accepter/abandonner/confirmer/noter (boutons, modales, confirmations) — seul
  l'affichage du badge « réservée » dans la grille de collection est couvert par cette spec.

## Critères d'acceptance

### Accepter

- [ ] `POST /trades/{id}/accept` par l'initiateur sur un trade `PENDING` → `204`, `initiator_accepted_at` renseigné,
      statut → `ONE_ACCEPTED`.
- [ ] `POST /trades/{id}/accept` par le répondant sur un trade `PENDING` → `204`, `respondent_accepted_at` renseigné,
      statut → `ONE_ACCEPTED`.
- [ ] `POST /trades/{id}/accept` par la seconde partie sur un trade `ONE_ACCEPTED` → `204`, l'autre colonne
      d'acceptation est renseignée, statut → `FULLY_ACCEPTED`.
- [ ] `POST /trades/{id}/accept` par une partie ayant déjà accepté → `409`, aucun changement.
- [ ] `POST /trades/{id}/accept` sur un trade `FULLY_ACCEPTED`, `COMPLETED`, `CLOSED` ou `ABANDONED` → `409`.
- [ ] `POST /trades/{id}/accept` par un utilisateur qui n'est ni l'initiateur ni le répondant → `403`.
- [ ] Lorsqu'un trade passe à `ONE_ACCEPTED`, tout autre trade actif (`PENDING` ou `ONE_ACCEPTED`) partageant au
      moins une carte (même clé composite + même `owner_user_id`) passe automatiquement à `ABANDONED`.
- [ ] Un trade `FULLY_ACCEPTED` partageant une carte avec le trade qui vient d'atteindre `ONE_ACCEPTED` n'est **pas**
      abandonné par cette cascade.
- [ ] Le passage `ONE_ACCEPTED` → `FULLY_ACCEPTED` (seconde acceptation) ne déclenche aucune cascade d'abandon.

### Abandonner

- [ ] `POST /trades/{id}/abandon` par l'initiateur ou le répondant, sur un trade `PENDING`, `ONE_ACCEPTED` ou
      `FULLY_ACCEPTED` → `204`, statut → `ABANDONED`.
- [ ] `POST /trades/{id}/abandon` sur un trade déjà `COMPLETED`, `CLOSED` ou `ABANDONED` → `409`.
- [ ] `POST /trades/{id}/abandon` par un utilisateur qui n'est ni l'initiateur ni le répondant → `403`.
- [ ] Après abandon d'un trade `ONE_ACCEPTED` ou `FULLY_ACCEPTED`, ses cartes ne sont plus comptées comme réservées
      (un autre trade portant sur la même carte peut atteindre `ONE_ACCEPTED` sans être bloqué par ce trade).

### Confirmer

- [ ] `POST /trades/{id}/confirm` par l'initiateur sur un trade `FULLY_ACCEPTED` → `204`,
      `initiator_confirmed_at` renseigné, statut reste `FULLY_ACCEPTED` (une seule partie a confirmé).
- [ ] `POST /trades/{id}/confirm` par la seconde partie → `204`, l'autre colonne de confirmation est renseignée,
      statut → `COMPLETED`.
- [ ] `POST /trades/{id}/confirm` par une partie ayant déjà confirmé → `409`, aucun changement.
- [ ] `POST /trades/{id}/confirm` sur un trade qui n'est pas `FULLY_ACCEPTED` (`PENDING`, `ONE_ACCEPTED`,
      `COMPLETED`, `CLOSED`, `ABANDONED`) → `409`.
- [ ] `POST /trades/{id}/confirm` par un utilisateur qui n'est ni l'initiateur ni le répondant → `403`.

### Noter

- [ ] `POST /trades/{id}/rate` avec `rating` entre 0 et 5, par l'initiateur sur un trade `COMPLETED` → `204`,
      `initiator_rating` renseigné, statut reste `COMPLETED` (l'autre partie n'a pas encore noté).
- [ ] `POST /trades/{id}/rate` par la seconde partie → `204`, `respondent_rating` renseigné, statut → `CLOSED`, note
      finale du trade = moyenne des deux notes.
- [ ] `POST /trades/{id}/rate` par une partie ayant déjà noté ce trade → `409`, aucun changement.
- [ ] `POST /trades/{id}/rate` sur un trade qui n'est pas `COMPLETED` → `409`.
- [ ] `POST /trades/{id}/rate` avec `rating` absent, non entier, négatif ou supérieur à 5 → `400`, aucun changement.
- [ ] `POST /trades/{id}/rate` par un utilisateur qui n'est ni l'initiateur ni le répondant → `403`.
- [ ] Un trade `COMPLETED` noté par une seule des deux parties reste `COMPLETED` (pas de clôture automatique).

### Commun

- [ ] Chacun des quatre endpoints requiert un token valide ; sans token → `401`.
- [ ] Chacun des quatre endpoints sur un `{trade_id}` inexistant → `404`.
- [ ] Les quatre endpoints sont documentés dans `doc/openapi.yml`.

### Réservation (`GET /collection`)

- [ ] Une carte de l'utilisateur courant engagée (en tant que propriétaire) dans un trade `ONE_ACCEPTED` →
      `GET /collection` (mode « ma collection ») renvoie `reserved: true` pour cette carte.
- [ ] Une carte de l'utilisateur courant engagée dans un trade `FULLY_ACCEPTED` → `reserved: true`.
- [ ] Une carte engagée uniquement dans un trade `PENDING`, `COMPLETED`, `CLOSED` ou `ABANDONED` → `reserved: false`.
- [ ] Une carte non engagée dans aucun trade → `reserved: false`.
- [ ] En mode recherche publique (`owner_count` présent, `collection_entry` absent), le champ `reserved` n'est pas
      renseigné (ou vaut `false`), quel que soit l'état réel des trades du propriétaire consulté.
- [ ] Le champ `reserved` est documenté dans `doc/openapi.yml`.
- [ ] La grille de collection (frontend) affiche le badge « Réservée » (icône cadenas) sur les cartes avec
      `reserved = true`, conformément au rendu de la maquette (`maquette/components.jsx`, `maquette/styles.css`).
