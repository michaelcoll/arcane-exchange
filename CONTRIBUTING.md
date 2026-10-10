# Contributing

The repository layout (one folder per app, shared files at the root) is described in
[ARCHITECTURE.md](ARCHITECTURE.md).

## Prerequisites

- [`mise`](https://mise.jdx.dev/) — the single entry point for local commands. It pins Node, pnpm, Swift and the
  Rust/iOS CLI tools, and exports the environment they rely on (`DATABASE_URL`, `BACKEND_PORT`, external API URLs).
  Run tasks through `mise run`, not `cargo`, `pnpm` or `sqlx` directly.
- A stable **Rust** toolchain (edition 2024).
- A **PostgreSQL 18** instance.
- For the iOS app only: **Xcode 26**.

## Setup

```bash
mise install                    # toolchains — the postinstall hook also installs the lefthook git hooks
mise run setup                  # install dependencies
cp .env.example .env            # then fill in your Clerk keys (see README › Configuration)
docker compose up -d postgres   # or use your own PostgreSQL
mise run migrate                # apply database migrations

mise run back                   # API on http://localhost:8080
mise run front                  # app on http://localhost:3000
```

The `postgres` service in `docker-compose.yml` stores its data under `/mnt/ssd/…`; point that volume at a local
folder if the path does not exist on your machine.

### iOS app

```bash
cp ios-app/Config/Local.xcconfig.example ios-app/Config/Local.xcconfig   # then fill in your team and Clerk key
mise run build-ios
mise run test-ios
```

See [`.agents/ios.instructions.md`](.agents/ios.instructions.md) for the details (XcodeGen, Metal toolchain, …).

## Useful tasks

`mise tasks` lists every task with its description. The ones you will need most:

| Task                     | What it does                                                                                  |
| ------------------------ | --------------------------------------------------------------------------------------------- |
| `mise run test-backend`  | Backend tests — also regenerates the frontend TypeScript bindings (`frontend/app/bindings/`)  |
| `mise run test-frontend` | Frontend tests                                                                                |
| `mise run sqlx-prepare`  | Regenerates the `backend/.sqlx` metadata — required after changing any SQL query (needs a DB) |
| `mise run rebuild-docs`  | Regenerates `docs/openapi.yml` and `docs/db.md`                                               |
| `mise run checks`        | Everything above plus the backend, frontend and iOS lints                                     |
| `mise run format`        | Formats the codebase                                                                          |

## Before pushing

The lefthook hooks run `mise run format` on every commit and `mise run checks` on every push. CI enforces the same
gates, notably:

- **`docs/openapi.yml` must be up to date** — after touching a controller or a DTO, run `mise run rebuild-docs` and
  commit the result.
- **SQL queries are checked at compile time** — after changing one, run `mise run sqlx-prepare` and commit `backend/.sqlx/`.
- **ESLint fails on warnings**, and SwiftLint runs in `--strict` mode.

## Commits and pull requests

- Use [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `refactor:`, `docs:`, `build:`,
  `chore:`, …), with an optional scope such as `feat(ios):` or `fix(web):`.
- Use the same convention for pull request titles: they are labelled automatically from them.
- Issues are tracked in [GitHub Issues](https://github.com/michaelcoll/arcane-exchange/issues).

## Documentation

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — layers, generated contracts, background jobs, clients, deployment. Read it
  before any structural change.
- [`GLOSSARY.md`](GLOSSARY.md) — the domain glossary.
- [`docs/adr/`](docs/adr) — architecture decisions.
- [`docs/openapi.yml`](docs/openapi.yml) — the HTTP API, generated from the backend.
- [`docs/db.md`](docs/db.md) — the database schema (ERD).
- [`AGENTS.md`](AGENTS.md) and [`.agents/`](.agents) — conventions per area (endpoints, frontend, iOS, database,
  design system, CI).
