# Architecture

Vue d'ensemble technique d'Arcane Exchange. Ce document donne les points structurants et les
décisions qui ne se devinent pas en lisant le code ; les détails (signatures, colonnes, endpoints)
se lisent directement dans les sources et dans les documents générés.

Pour le vocabulaire métier, voir [CONTEXT.md](CONTEXT.md) ; pour les décisions de conception,
[docs/adr/](docs/adr/).

## Vue d'ensemble

Un backend Rust unique sert trois clients via une seule API HTTP :

```
                       ┌──────────────────────────┐
frontend-vue (Nuxt SPA)│                          │  Postgres
ios-app (SwiftUI)  ────┤  backend Rust (axum)     ├── (SQLx, migrations au démarrage)
                       │                          │
                       └────────────┬─────────────┘
                                    │ appels sortants
                       Cardmarket · Scryfall · Gatherer · EdhRec · Clerk (JWKS)
```

- **Backend** : `src/ae/`, binaire `ae`, Axum + SQLx + Postgres.
- **Web** : `frontend-vue/`, Nuxt 4 en **SPA** (`ssr: false`) ; son serveur Nitro sert les assets
  et **proxifie `/api/v1/**` vers le backend** — le navigateur ne parle jamais au backend en direct.
- **iOS** : `ios-app/`, SwiftUI, iOS 18+, Swift 6 strict concurrency, iPhone seulement.
- **Base** : un seul schéma Postgres, migrations SQLx dans `migrations/`, appliquées au démarrage du
  backend (pas d'étape de déploiement séparée).

## Backend — Clean Architecture

Trois couches sous `src/ae/`, dépendances strictement unidirectionnelles
(`infrastructure → application → domain`) :

| Couche            | Contenu                                                                                                                          |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `domain/`         | Types métier purs, invariants, erreurs fonctionnelles. Aucune dépendance technique.                                              |
| `application/`    | `use_case.rs` (traits = contrat entrant), `repository.rs` / `caller.rs` (traits = ports sortants), `service/` (implémentations). |
| `infrastructure/` | `adapter_in/` (contrôleurs HTTP + DTO), `adapter_out/` (`repository/` SQLx, `caller/` HTTP).                                     |

Points structurants :

- **Ports & adapters** : chaque service ne connaît que des traits. Tout est injecté en
  `Arc<dyn Trait>` et câblé au seul endroit qui connaît le monde concret, `infrastructure.rs`
  (`create_infra`) — c'est le point d'entrée à lire pour comprendre qui dépend de qui.
- **`AppState`** est le conteneur d'injection : un champ par use case, partagé par tous les routeurs.
- **Un fichier = un cas d'usage** côté `application/service/`, et un dossier par ressource HTTP côté
  `adapter_in/` (`controller.rs`, `dto.rs`, `tests.rs`).
- **Erreurs** : `AppError` est une façade sur trois familles — fonctionnelle (métier/validation),
  authentification, infrastructure. La famille détermine le code HTTP ; `?` traverse les couches
  grâce aux `From` par famille.
- **Configuration** : lue une seule fois au démarrage dans `config.rs` (`Config::from_env()`). Aucun
  autre module ne lit l'environnement.

## Contrats et génération de code

L'API est le point de synchronisation entre les trois applications, et elle est **générée depuis le
backend**, jamais écrite à la main :

- `docs/openapi.yml` est produit par le binaire `generate-openapi` à partir des annotations utoipa
  des contrôleurs. La CI vérifie qu'il est à jour.
- Le **client Swift** (`ios-app/APIClient/`) est généré depuis ce même `docs/openapi.yml` et commité.
- Les **types TypeScript** du front (`frontend-vue/app/bindings/`) sont générés par `ts-rs` lors de
  la compilation des tests backend.
- `docs/db.md` est l'ERD généré du schéma.

Conséquence pratique : **tout changement d'API commence dans le backend**, puis se propage par
régénération. Modifier un modèle côté client est toujours une erreur.

## Données

- **Postgres seul** : pas de cache externe, pas de broker. Les files d'attente sont en mémoire
  (canaux Tokio) et les agrégats coûteux sont des vues matérialisées.
- **Vues matérialisées** pour les lectures chaudes (prix courants, collection valorisée) : elles sont
  rafraîchies explicitement après chaque import, jamais en continu. Elles sont donc _stale by
  design_, et en lecture seule.
- **Une vue non matérialisée** dérive, pour chaque joueur, la quantité réellement proposée à
  l'échange à partir de sa visibilité, de ses binders ouverts et de ses filtres de rareté. C'est la
  seule source de l'exposition d'une carte à un tiers.
- **Prix en centimes**, entiers, partout — jamais de flottant.
- **SQLx en mode vérifié à la compilation** : les requêtes sont validées contre une base réelle, et
  la métadonnée `.sqlx/` est commitée pour que les builds release/CI se fassent hors ligne.

## Traitements asynchrones

Tout tourne dans le même processus que l'API, sans ordonnanceur externe :

- **Import de collection** : l'endpoint parse le CSV, crée l'import en base, puis remet le travail à
  un worker via un canal Tokio et répond immédiatement. Le client suit l'avancement en interrogeant
  l'état de l'import. Au démarrage, les imports restés actifs d'un process précédent sont marqués en
  échec.
- **Enrichissement des cartes** (identifiants Cardmarket via Scryfall, identifiants Gatherer) :
  workers alimentés par des canaux, avec un ensemble de déduplication partagé entre l'émetteur et le
  worker pour ne pas retraiter une carte déjà en file.
- **Import des prix** : tâche planifiée (cron in-process) toutes les 12 heures.

Corollaire : le backend est **stateful en mémoire** (files, dédup). Il n'est pas conçu pour tourner
en plusieurs répliques en l'état.

## Authentification

- **Clerk est la seule autorité d'authentification.** Le backend ne stocke aucun mot de passe et
  n'émet aucun jeton.
- Les clients obtiennent un JWT Clerk et l'envoient en `Authorization: Bearer`. Le backend le valide
  avec les clés publiques (JWKS) de l'instance Clerk.
- La table des utilisateurs est un **miroir local** du strict nécessaire (identifiant, pseudo), pas
  une source de vérité.
- Un extracteur de requête porte l'authentification : **un handler qui le déclare est protégé, un
  handler qui ne le déclare pas est public.** Seuls la maintenance et l'autocomplétion d'utilisateurs
  sont publics.

## Clients

Les deux clients consomment la même API et partagent le même langage métier, mais **ne partagent pas
de code**. La maquette (non versionnée) sert de référence visuelle commune ; chaque client l'adapte à
ses idiomes plutôt que de la transposer littéralement.

- **Web** — Nuxt 4 / Vue 3 / Tailwind. Les appels API passent tous par un composable unique qui
  injecte le jeton ; un composable de service par ressource. Thème sombre par défaut, jetons de
  design en variables CSS.
- **iOS** — SwiftUI + `@Observable`, un dossier par écran (`Features/<Feature>/` avec sa vue et son
  view model), pas de Combine. Le projet Xcode est **généré** (XcodeGen) et non versionné : la source
  de vérité est `ios-app/project.yml` et les `.xcconfig`.

## Outillage et déploiement

- **mise** est l'unique point d'entrée des commandes locales (toolchains + tâches). Rien ne s'exécute
  directement avec `cargo` / `pnpm` / `xcodebuild` dans le flux normal.
- **CI GitHub Actions** : trois pipelines indépendants (backend, frontend, iOS), plus la publication
  d'images. Les tâches iOS sont volontairement hors des tâches globales, pour ne pas déclencher
  l'outillage Xcode à chaque commit.
- **Déploiement** : deux images Docker (backend distroless, frontend Node/Nitro) orchestrées par
  `docker-compose.yml` avec Postgres. Sentry est branché côté backend et côté frontend.
