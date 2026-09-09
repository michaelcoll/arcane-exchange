# Spec : Import de cartes asynchrone et optimisé

## Contexte

L'import de collection ManaBox (`POST /collection/import`) est aujourd'hui entièrement synchrone : le client envoie le
CSV, la requête HTTP reste ouverte pendant tout le traitement, et le backend répond `200 { message: "Cards imported
successfully" }` une fois terminé.

Deux problèmes :

1. **Performance** — `ImportCardService` boucle carte par carte et exécute, pour chaque ligne, une vérification/insertion
   de `set_name` puis un upsert de `card` et de `collection_entry`. Sur une collection de plusieurs milliers de cartes,
   cela représente autant d'aller-retours SQL, et le temps de traitement croît linéairement.
2. **Expérience utilisateur** — le client (modal d'import du frontend Vue) reste bloqué sur un spinner sans progression,
   et un import long expose au timeout de la requête HTTP (proxy, navigateur). L'app iOS ne propose pas encore d'écran
   d'import.

## Objectif

- Rendre l'import **asynchrone** : la requête d'import rend la main immédiatement, le traitement se poursuit côté
  serveur.
- Exposer le **statut de l'import** (état, progression, erreurs) via l'API, pour que les clients affichent une barre de
  progression et un compte-rendu.
- Garantir **un seul import en cours à la fois par utilisateur**.
- **Accélérer** significativement le traitement en supprimant le pattern « une requête SQL par carte ».

## Solution

### Suivi d'import en base

L'état des imports est persisté en base dans une table dédiée (nouvelle migration, un adapter dédié conformément à la
convention « une table = un adapter »), pour survivre à un redémarrage du backend et permettre la consultation de
l'historique.

Chaque import porte au minimum :

- un identifiant propre (UUID) ;
- le propriétaire (`user_id`, FK vers `users`) ;
- un état parmi : `pending`, `running`, `completed`, `failed` ;
- des compteurs de progression : nombre total de lignes à traiter et nombre de lignes déjà traitées ;
- les erreurs rencontrées (voir « Remontée des erreurs ») ;
- les horodatages de création et de fin.

### Déclenchement asynchrone

- `POST /collection/import` conserve son contrat d'entrée (corps `text/plain`, CSV ManaBox, limite 10 Mo) et
  l'authentification `AuthenticatedUser`.
- Le handler valide l'encodage UTF-8 et le **parsing du CSV de façon synchrone** : une erreur de format doit être
  signalée immédiatement à l'utilisateur, pas découverte plus tard dans un statut d'import.
- Si le parsing réussit, l'import est créé en base à l'état `pending` et le traitement effectif est délégué à une tâche
  de fond. L'endpoint répond alors `202 Accepted` avec l'identifiant de l'import.
- Le mécanisme de tâche de fond réutilise l'infrastructure existante (workers Tokio câblés dans `infrastructure.rs`, à
  l'image des workers CardMarket ID et Gatherer ID) plutôt que d'introduire une nouvelle brique.
- Le traitement de fond conserve les étapes fonctionnelles actuelles, dans le même ordre : suppression des entrées de
  collection existantes de l'utilisateur, écriture des sets / cartes / entrées de collection, purge des binders
  orphelins, enqueue des identifiants CardMarket et Gatherer manquants, puis rafraîchissement des vues matérialisées
  (`mv_last_cardmarket_prices` puis `mv_card_prices`).
- La progression est mise à jour au fil de l'eau pendant le traitement, de sorte qu'un appel au statut pendant un import
  long renvoie une valeur qui progresse.

### Unicité par utilisateur

- Un utilisateur ne peut avoir qu'un seul import à l'état `pending` ou `running` à un instant donné. L'utilisateur est
  celui identifié par le jeton Bearer de la requête (extracteur `AuthenticatedUser`).
- Une nouvelle demande d'import alors qu'un import de **cet utilisateur** est déjà en cours est refusée avec un
  `409 Conflict`, sans annuler ni perturber l'import en cours.
- La contrainte est portée par la base (et pas seulement par une vérification applicative) pour rester correcte en cas
  de requêtes concurrentes.
- Les imports d'utilisateurs différents restent parallélisables.

### Consultation du statut

Deux endpoints, tous deux authentifiés et limités aux imports de l'utilisateur courant :

- `GET /collection/import/{id}` — détail d'un import : état, progression (lignes traitées / total), erreurs,
  horodatages.
- `GET /collection/import` — liste des imports de l'utilisateur, du plus récent au plus ancien.

Les deux endpoints sont documentés via utoipa et donc répercutés dans `doc/openapi.yml`, qui reste la source de vérité
pour les clients générés.

### Remontée des erreurs

- Les erreurs de parsing d'une ligne CSV (set inconnu, quantité invalide, prix illisible, date invalide…) sont
  collectées et rattachées à l'import, avec le numéro de ligne et la raison, de manière consultable via l'endpoint de
  détail.
- Le nombre d'erreurs remontées est borné pour éviter qu'un fichier entièrement invalide ne fasse exploser la taille de
  la réponse ; au-delà de cette borne, seul le compte total est conservé.

### Historique et rétention

- L'historique des imports est conservé par utilisateur, limité aux **10 imports les plus récents** : au-delà, les plus
  anciens sont supprimés.
- La purge s'applique aux imports terminés (`completed` ou `failed`) uniquement ; un import en cours n'est jamais purgé.

### Optimisation des écritures

- Les écritures de `set_name`, `card` et `collection_entry` doivent se faire par **insertions groupées** (batchs), et
  non par une requête par carte. Les sémantiques actuelles sont conservées : upsert `ON CONFLICT … DO UPDATE`, identité
  de carte composite `(set_code, collector_number, language_code, foil)`, prix en centimes (entiers).
- La déduplication et le calcul de moyenne pondérée des prix d'achat effectués par `parse_service` restent inchangés.
- Le comportement fonctionnel de l'import (résultat final en base) doit être strictement identique à l'existant : seule
  la façon d'écrire change.

### Reprise après redémarrage

Au démarrage du backend, tout import resté à l'état `pending` ou `running` est marqué `failed` avec une raison
explicite (interruption du serveur). Il n'y a pas de reprise automatique : l'utilisateur relance son import. Ce
traitement libère mécaniquement le verrou d'unicité.

### Clients

**Frontend Vue** — la modal d'import (`app/pages/collection/index.vue` /
`app/composables/useCollectionService.ts`) est adaptée :

- l'envoi du CSV reçoit un identifiant d'import au lieu d'attendre la fin du traitement ;
- le statut est interrogé périodiquement tant que l'import est `pending` ou `running`, avec affichage d'une progression
  chiffrée ;
- à la fin, la collection est rafraîchie comme aujourd'hui, et le compte-rendu (nombre de cartes importées, erreurs de
  lignes) est affiché ;
- un `409` est présenté comme « un import est déjà en cours » avec le statut de cet import.

**iOS** — un écran d'import est ajouté à l'app, s'appuyant sur le client `APIClient` régénéré depuis `doc/openapi.yml` :
sélection du fichier CSV, envoi, suivi de progression et compte-rendu, avec le même traitement du `409`.

## Cas d'erreurs

- **Corps non UTF-8 ou CSV non conforme au format ManaBox** : `400 Bad Request` immédiat, aucun import n'est créé en
  base.
- **Fichier dépassant 10 Mo** : rejet comme aujourd'hui, aucun import créé.
- **Requête non authentifiée ou jeton invalide** : `401 Unauthorized`.
- **Import déjà en cours pour l'utilisateur** : `409 Conflict`, l'import en cours n'est ni annulé ni modifié.
- **Consultation d'un import inexistant, ou appartenant à un autre utilisateur** : `404 Not Found` (aucune fuite
  d'information sur l'existence de l'import d'autrui).
- **Échec du traitement de fond** (erreur base de données, échec du rafraîchissement des vues matérialisées) : l'import
  passe à `failed` avec un message d'erreur consultable ; l'utilisateur peut relancer un import.
- **Lignes individuellement invalides** : elles sont ignorées et collectées comme erreurs de ligne ; l'import se termine
  malgré tout en `completed` si au moins une ligne a pu être traitée.
- **Aucune ligne exploitable** : l'import se termine en `failed` avec le détail des erreurs, et la collection existante
  de l'utilisateur n'est pas vidée.
- **Redémarrage du backend pendant un import** : l'import est marqué `failed` au démarrage suivant.
- **Polling du statut après purge de l'historique** : `404 Not Found`.

## Critères d'acceptance

- [ ] Étant donné un utilisateur authentifié et un CSV ManaBox valide, quand il appelle `POST /collection/import`, alors
      la réponse est `202 Accepted`, contient l'identifiant de l'import, et est rendue avant la fin du traitement.
- [ ] Étant donné un CSV mal formé ou non UTF-8, quand il est envoyé, alors la réponse est `400 Bad Request` et aucun
      import n'apparaît dans `GET /collection/import`.
- [ ] Étant donné un import venant d'être lancé, quand on appelle `GET /collection/import/{id}`, alors la réponse
      contient un état parmi `pending`, `running`, `completed`, `failed`, le nombre de lignes traitées et le nombre
      total de lignes.
- [ ] Étant donné un import long en cours, quand on appelle le statut à deux instants espacés, alors le nombre de lignes
      traitées de la seconde réponse est supérieur ou égal à celui de la première.
- [ ] Étant donné un import terminé avec succès, quand on consulte son statut, alors l'état est `completed`, le nombre
      de lignes traitées est égal au nombre total, et une date de fin est renseignée.
- [ ] Étant donné un import déjà `pending` ou `running` pour un utilisateur, quand ce même utilisateur relance
      `POST /collection/import`, alors la réponse est `409 Conflict` et l'import en cours reste dans le même état.
- [ ] Étant donné deux utilisateurs distincts, quand chacun lance un import, alors les deux imports sont acceptés en
      `202` et se déroulent tous les deux jusqu'à `completed`.
- [ ] Étant donné deux requêtes d'import concurrentes du même utilisateur envoyées simultanément, quand elles sont
      traitées, alors exactement une reçoit `202` et l'autre `409`.
- [ ] Étant donné un import appartenant à un autre utilisateur, quand on appelle `GET /collection/import/{id}` avec son
      identifiant, alors la réponse est `404 Not Found`.
- [ ] Étant donné un utilisateur ayant réalisé plusieurs imports, quand il appelle `GET /collection/import`, alors il
      obtient uniquement ses propres imports, triés du plus récent au plus ancien.
- [ ] Étant donné un utilisateur ayant réalisé 11 imports, quand on consulte la liste, alors seuls les 10 plus récents
      sont présents.
- [ ] Étant donné un CSV contenant des lignes invalides et des lignes valides, quand l'import se termine, alors l'état
      est `completed`, les lignes valides sont en base, et les lignes invalides sont listées dans les erreurs de
      l'import avec leur numéro de ligne.
- [ ] Étant donné un CSV dont aucune ligne n'est exploitable, quand l'import se termine, alors l'état est `failed` et la
      collection de l'utilisateur est inchangée.
- [ ] Étant donné un import à l'état `running` en base, quand le backend redémarre, alors cet import passe à `failed` et
      un nouvel import du même utilisateur est accepté.
- [ ] Étant donné un import de N cartes, quand il s'exécute, alors le nombre de requêtes SQL d'écriture sur `card`,
      `collection_entry` et `set_name` ne croît pas linéairement avec N (écritures groupées vérifiables par test).
- [ ] Étant donné le même CSV importé avant et après ce changement, quand on compare le contenu de `card`,
      `collection_entry`, `set_name` et `trading_binders`, alors le résultat est identique.
- [ ] Étant donné un import terminé, quand on interroge `mv_card_prices` pour l'utilisateur, alors les prix reflètent la
      collection importée (vues matérialisées rafraîchies).
- [ ] `doc/openapi.yml` décrit les endpoints `POST /collection/import` (202, 400, 401, 409),
      `GET /collection/import/{id}` (200, 401, 404) et `GET /collection/import` (200, 401).
- [ ] Étant donné la modal d'import du frontend Vue, quand un import est lancé, alors une progression chiffrée s'affiche
      et se met à jour jusqu'à la fin, puis la collection est rafraîchie et le compte-rendu affiché.
- [ ] Étant donné un import déjà en cours, quand l'utilisateur en relance un depuis le frontend Vue, alors un message
      « import déjà en cours » est affiché avec l'état de l'import courant.
- [ ] Étant donné l'app iOS, quand l'utilisateur sélectionne un CSV ManaBox et lance l'import, alors la progression est
      affichée jusqu'à la fin et le compte-rendu (cartes importées, erreurs) est présenté.
