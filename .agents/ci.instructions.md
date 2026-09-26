# GitHub Actions CI/CD Guide

Sept workflows dans `.github/workflows/`. Déploiement et images : [ARCHITECTURE.md](../ARCHITECTURE.md) ·
équivalents locaux : [mise.instructions.md](mise.instructions.md).

| Workflow                 | Déclencheur                | Jobs                                                     |
| ------------------------ | -------------------------- | -------------------------------------------------------- |
| `lint-test-backend.yml`  | chaque push                | lint · test (+ Codecov) · build-offline · check-openapi  |
| `lint-test-frontend.yml` | chaque push                | format · typecheck · eslint · build-production           |
| `lint-test-ios.yml`      | push touchant `ios-app/**` | lint (SwiftFormat + SwiftLint `--strict`) · build & test |
| `build-push.yml`         | push sur `main`, release   | images backend + frontend vers GHCR ; release → Sentry   |
| `automerge.yml`          | pull_request               | auto-merge des PR dependabot patch/minor                 |
| `pr-label.yml`           | PR ouverte/éditée          | labels conventionnels depuis le titre                    |
| `clean-cache.yml`        | PR fermée                  | purge du cache runner                                    |

Points qui ne se lisent pas dans les YAML :

- **`check-openapi` est un gate** : il régénère `docs/openapi.yml` et échoue s'il diffère. Toute modification de
  contrôleur ou de DTO impose `mise run rebuild-docs` + commit du fichier.
- **Aucun job Vitest** : les tests frontend ne tournent qu'en local (`mise run test-frontend`).
- **Les noms des jobs sont des status checks requis** par le ruleset de `main` (ex. « Typecheck ») : renommer
  ou supprimer un job impose de mettre à jour le ruleset dans le même temps, sinon plus rien ne se merge.
- **ESLint échoue aussi sur les warnings** (`--max-warnings 0`). `typecheck` et `eslint` sont deux jobs distincts
  en CI, mais `pnpm lint` / `mise run lint-frontend` enchaînent les deux en local.
- **Pas de build « development »** : `nuxi build` produit toujours un build de production et la config ne
  déclare aucun override `$development`. L'ancien job `build-dev` (`--configuration development`, reliquat
  d'Angular CLI) construisait en fait une app vide dans `frontend-vue/development/` ; il a été supprimé.
- **iOS** : seul pipeline sur macOS, `macos-26` épinglé car `clerk-ios` >= 1.2.0 exige Xcode 26
  (`swift-tools-version: 6.2`). Simulateur uniquement — pas de signature, de provisioning ni de TestFlight. Le job
  `test` ne construit pas dans un job séparé : `xcodebuild test` build déjà l'app et son bundle de tests.
