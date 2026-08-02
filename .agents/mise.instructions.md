# Mise & Development Workflow

## Quick Reference

All local commands are run through **mise** (task runner):

```
mise install           # One-time: install toolchains (Rust, Node, pnpm, etc.)
mise run setup         # Full dev setup: clean + install frontend deps
```

## Command Summary

| Action                     | Command                   | Alias         |
|----------------------------|---------------------------|---------------|
| **All checks**             | `mise run checks`         | —             |
| **Backend server**         | `mise run back`           | —             |
| **Frontend dev server**    | `mise run front`          | —             |
| **Backend tests**          | `mise run test-backend`   | —             |
| **Frontend tests**         | `mise run test-frontend`  | —             |
| **Backend lint**           | `mise run lint-backend`   | —             |
| **Frontend lint**          | `mise run lint-frontend`  | —             |
| **Format code front/back** | `mise run format`         | `mise run f`  |
| **OpenAPI gen**            | `mise run openapi`        | `mise run o`  |
| **DB migrations**          | `mise run migrate`        | —             |
| **SQLx metadata**          | `mise run sqlx-prepare`   | —             |
| **Clean artifacts**        | `mise run clean`          | —             |
| **Upgrade deps**           | `mise run upgrade`        | —             |
| **Build backend**          | `mise run build-backend`  | `mise run bb` |
| **Build frontend**         | `mise run build-frontend` | `mise run bf` |

Note: there is no combined `mise run lint`. Backend and frontend lint are separate tasks (`lint-backend`,
`lint-frontend`); only `format` runs both front and back together.

## Detailed Commands

### Build

- **Backend**: `mise run build-backend` (= `mise run bb`), i.e. `cargo build`. In production/Docker:
  `SQLX_OFFLINE=true cargo build --release`
- **Frontend**: `mise run build-frontend` (= `mise run bf`), i.e. `pnpm build` in `frontend-vue`
  (depends on `install-frontend-deps`)

### Test

- **Backend**: `mise run test-backend`, i.e. `cargo nextest run --status-level slow`
- **Frontend**: `mise run test-frontend`, i.e. `pnpm run test` in `frontend-vue` (Vitest)

### Lint

- **Backend**: `mise run lint-backend`
    1. `lint-clippy`: `cargo clippy --locked --workspace --all-features --all-targets -- -A dead_code -D clippy::all`
    2. `lint-sqlx`: `cargo sqlx prepare --check` (depends on `sqlx-prepare`; validates SQL queries against the DB)
- **Frontend**: `mise run lint-frontend`, i.e. `pnpm lint` in `frontend-vue`

### Format

- **Backend**: `format-backend` → `cargo fmt` (rustfmt)
- **Frontend**: `format-frontend` → `pnpm format:fix` in `frontend-vue`
- **Both**: `mise run format` (= `mise run f`) — always use this, never call `cargo fmt` / `pnpm format:fix` directly

### OpenAPI

```
mise run openapi        # Generates doc/openapi.yml, then runs format-frontend
```

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
