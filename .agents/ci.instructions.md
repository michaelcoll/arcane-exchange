# GitHub Actions CI/CD Guide

Huit workflows dans `.github/workflows/`. Déploiement et images : [ARCHITECTURE.md](../ARCHITECTURE.md) ·
équivalents locaux : [mise.instructions.md](mise.instructions.md).

| Workflow                 | Déclencheur                | Jobs                                                     |
| ------------------------ | -------------------------- | -------------------------------------------------------- |
| `lint-test-backend.yml`  | chaque push                | lint · test (+ Codecov) · build-offline · check-openapi  |
| `lint-test-frontend.yml` | chaque push                | format · typecheck · eslint · build-production           |
| `lint-test-ios.yml`      | push touchant `ios-app/**` | lint (SwiftFormat + SwiftLint `--strict`) · build & test |
| `build-push.yml`         | push sur `main`, release   | images backend + frontend vers GHCR ; release → Sentry   |
| `announce-release.yml`   | release publiée, manuel    | tests du script · annonce Discord `#release` (dry run)   |
| `automerge.yml`          | pull_request               | auto-merge des PR dependabot patch/minor                 |
| `pr-label.yml`           | PR ouverte/éditée          | labels conventionnels depuis le titre                    |
| `clean-cache.yml`        | PR fermée                  | purge du cache runner                                    |

Points qui ne se lisent pas dans les YAML :

- **Labels de PR** : `pr-label.yml` ne pose que `feat` et `fix`, depuis le préfixe ancré du titre
  (`^type(scope)?!?:`). Les notes de release (`.github/release.yml`) ont quatre catégories, dans l'ordre :
  🎁 Features (`feat`), 🔧 Fixes (`fix`), 📦 Dependencies (`dependencies`), 🧹 Maintenance (attrape-tout `*`).
  Règle de nommage et scopes : [AGENTS.md](../AGENTS.md#pr-titles).

- **`check-openapi` est un gate** : il régénère `docs/openapi.yml` et échoue s'il diffère. Toute modification de
  contrôleur ou de DTO impose `mise run rebuild-docs` + commit du fichier.
- **Aucun job Vitest** : les tests frontend ne tournent qu'en local (`mise run test-frontend`).
- **Les noms des jobs sont des status checks requis** par le ruleset de `main` (ex. « Typecheck ») : renommer
  ou supprimer un job impose de mettre à jour le ruleset dans le même temps, sinon plus rien ne se merge.
- **ESLint échoue aussi sur les warnings** (`--max-warnings 0`). `typecheck` et `eslint` sont deux jobs distincts
  en CI, mais `pnpm lint` / `mise run lint-frontend` enchaînent les deux en local.
- **Pas de build « development »** : `nuxi build` produit toujours un build de production et la config ne
  déclare aucun override `$development`. L'ancien job `build-dev` (`--configuration development`, reliquat
  d'Angular CLI) construisait en fait une app vide dans `frontend/development/` ; il a été supprimé.
- **`docs/openapi.yml` reste à la racine** alors que `check-openapi` lance le générateur depuis `backend/` : il
  résout sa sortie depuis le manifeste du crate, pas depuis le dossier courant.
- **iOS** : seul pipeline sur macOS, `macos-26` épinglé car `clerk-ios` >= 1.2.0 exige Xcode 26
  (`swift-tools-version: 6.2`). Simulateur uniquement — pas de signature, de provisioning ni de TestFlight. Le job
  `test` ne construit pas dans un job séparé : `xcodebuild test` build déjà l'app et son bundle de tests.
- **Pas de `restore-keys` sur le cache DerivedData iOS** : une correspondance partielle restaurerait des paquets
  compilés par un autre Xcode, que le build incrémental réutiliserait.
