# Mise & Development Workflow

## Quick Reference

All local commands are run through **mise** (task runner):

```
mise install           # One-time: install toolchains (Rust, Node, pnpm, etc.)
mise run setup         # Full dev setup: clean + install frontend deps
```

## Command Summary

| Action                      | Command                     | Alias         |
| --------------------------- | --------------------------- | ------------- |
| **All checks**              | `mise run checks`           | —             |
| **Backend server**          | `mise run back`             | —             |
| **Frontend dev server**     | `mise run front`            | —             |
| **Backend tests**           | `mise run test-backend`     | —             |
| **Backend coverage**        | `mise run coverage-backend` | —             |
| **Frontend tests**          | `mise run test-frontend`    | —             |
| **Backend lint**            | `mise run lint-backend`     | —             |
| **Frontend lint**           | `mise run lint-frontend`    | —             |
| **Format code front/back**  | `mise run format`           | `mise run f`  |
| **Docs gen (OpenAPI + DB)** | `mise run rebuild-docs`     | —             |
| **DB migrations**           | `mise run migrate`          | —             |
| **SQLx metadata**           | `mise run sqlx-prepare`     | —             |
| **Clean artifacts**         | `mise run clean`            | —             |
| **Upgrade deps**            | `mise run upgrade`          | —             |
| **Build backend**           | `mise run build-backend`    | `mise run bb` |
| **Build frontend**          | `mise run build-frontend`   | `mise run bf` |

Note: there is no combined `mise run lint`. Backend and frontend lint are separate tasks (`lint-backend`,
`lint-frontend`); only `format` runs both front and back together.

### iOS

`build-ios`, `test-ios`, `lint-ios` are standalone tasks (`dir = "ios-app"`) — **deliberately not** wired into
`checks`/`format`/`setup`/`clean`, since `lefthook.yml` has no per-folder scoping and would otherwise run Xcode
tooling on every commit/push regardless of what changed:

| Action    | Command              | Alias         |
| --------- | -------------------- | ------------- |
| **Build** | `mise run build-ios` | `mise run bi` |
| **Test**  | `mise run test-ios`  | —             |
| **Lint**  | `mise run lint-ios`  | —             |

All depend on the hidden `generate-ios` task (`xcodegen generate`). `xcodegen`, `swiftlint`, `swiftformat` are
pinned in `[tools]`; Xcode itself is not mise-managed (system install required).

#### Local configuration

`ios-app/Config/Local.xcconfig` is git-ignored and holds the two per-developer values — `DEVELOPMENT_TEAM` and
`CLERK_PUBLISHABLE_KEY`. First checkout:

```sh
cp ios-app/Config/Local.xcconfig.example ios-app/Config/Local.xcconfig
```

`Shared.xcconfig` pulls it in with `#include?` (optional include) and declares an empty `CLERK_PUBLISHABLE_KEY`
default, so `build-ios` / `test-ios` — and CI, which never writes that file — still pass without it. Only sign-in
needs the real key. `xcodegen` maps it into `Info.plist`, where `AppConfig.clerkPublishableKey` reads it.

## Detailed Commands

### Build

- **Backend**: `mise run build-backend` (= `mise run bb`), i.e. `cargo build`. In production/Docker:
  `SQLX_OFFLINE=true cargo build --release`
- **Frontend**: `mise run build-frontend` (= `mise run bf`), i.e. `pnpm build` in `frontend-vue`
  (depends on `install-frontend-deps`)

### Test

- **Backend**: `mise run test-backend`, i.e. `cargo nextest run --status-level slow`. **Side-effect** : le derive
  `ts-rs` est exécuté lors de la compilation, ce qui régénère les fichiers TypeScript dans `frontend-vue/app/bindings/`
  (ex. `RarityCode.ts`, `CollectionParams.ts`, etc.). Toute modification d'un enum DTO (`ToSchema` + `TS`) nécessite de
  relancer `mise run test-backend`
  pour que les bindings front end soient à jour.
- **Frontend**: `mise run test-frontend`, i.e. `pnpm run test` in `frontend-vue` (Vitest)

### Coverage

- **Backend**: `mise run coverage-backend`, i.e.
  `cargo llvm-cov nextest --status-level slow --locked --workspace --all-features --bin ae --tests --ignore-filename-regex "main.rs|infrastructure.rs|generate_openapi.rs" --lcov --output-path lcov.info`.
  Runs the full backend suite under `nextest` (needed so coverage instrumentation doesn't OOM — plain
  `cargo llvm-cov`/`cargo test` runs every test as a thread in one process, which is too heavy combined with the many
  `#[sqlx::test]` integration tests; `nextest` isolates each test in its own process instead). Prints a per-file summary
  table (regions/functions/lines %) and writes `lcov.info`
  (gitignored) with per-line hit counts, so you can find exactly which lines are missing for a given file, e.g.
  `awk '/SF:.*trade\/controller.rs/{f=1} f{print} /end_of_record/{if(f) exit}' lcov.info | grep '^DA:.*,0'`
  (`DA:<line>,0` = uncovered line).

### Lint

- **Backend**: `mise run lint-backend`
  1. `lint-clippy`: `cargo clippy --locked --workspace --all-features --all-targets -- -A dead_code -D clippy::all`
  2. `lint-sqlx`: `cargo sqlx prepare --check` (depends on `sqlx-prepare`; validates SQL queries against the DB)
- **Frontend**: `mise run lint-frontend`, i.e. `pnpm lint` in `frontend-vue`

### Format

- **Backend**: `format-backend` → `cargo fmt` (rustfmt)
- **Frontend**: `format-frontend` → `pnpm format:fix` in `frontend-vue`
- **Both**: `mise run format` (= `mise run f`) — always use this, never call `cargo fmt` / `pnpm format:fix` directly

### Docs (OpenAPI + DB schema)

```
mise run rebuild-docs   # rebuild-openapi-doc (cargo run --bin generate-openapi) + rebuild-db-doc (erdify → doc/db.md)
```

Both sub-tasks are hidden (not directly listed by `mise run`); run individually via `mise run rebuild-openapi-doc`
or `mise run rebuild-db-doc` if you only need one. Neither runs formatting — `mise run rebuild-docs` is itself one
of the dependencies of `mise run checks`, alongside `sqlx-prepare`, tests and lints.

### Database

```
mise run migrate        # Applies SQLx migrations (sqlx migrate run)
mise run sqlx-prepare    # Generates SQLx metadata (run after modifying queries)
```

### Clean & Setup

```
mise run clean          # clean-backend (cargo clean) + clean-frontend (rm .nuxt .output node_modules)
mise run setup          # clean + install-frontend-deps (pnpm install in frontend-vue)
```

### Upgrade

```
mise run upgrade        # upgrade-backend (cargo update) + upgrade-frontend (pnpm update)
```
