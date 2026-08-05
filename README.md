# arcane-exchange

[![codecov](https://codecov.io/gh/michaelcoll/arcane-exchange/graph/badge.svg?token=b2Wlmg2WX3)](https://codecov.io/gh/michaelcoll/arcane-exchange)

## Overview

A web application tracking card collection prices (e.g., Magic: The Gathering). It fetches and stores prices via a REST
API consumed by an Angular frontend.

## Requirements

- **Rust** (edition 2024, stable toolchain)
- **Node.js** >= 24 & **pnpm** >= 11 (Frontend)
- **PostgreSQL** 18
- **Docker/Compose** (Optional)
- [`mise`](https://mise.jdx.dev/) — Task runner & toolchain manager

## Setup

### Docker Compose (Recommended)

1. Copy and configure `.env` from `example-files/`.
2. Start services: `docker compose up -d`
   - Backend API: <http://localhost:8080>
   - Frontend: <http://localhost:9797>

### Local Development

1. Start PostgreSQL service (if not using Compose).
2. Copy/configure `.env`.
3. Install SDKs and tools `mise install`.
4. Setup project: `mise run setup`.
5. Run backend: `mise run back`. (API on <http://localhost:8080>)
6. Run frontend: `mise run front`. (app on <http://localhost:4200>)

## Environment Variables

| Variable                 | Default                                           | Description                                      |
| ------------------------ | ------------------------------------------------- | ------------------------------------------------ |
| `DATABASE_URL`           | `postgres://postgres:password@localhost/postgres` | PostgreSQL connection string                     |
| `BACKEND_PORT`           | `8080`                                            | Backend API port                                 |
| `CLERK_FRONTEND_API_URL` | _(required)_                                      | Clerk frontend API URL for JWT validation (JWKS) |

> **Authentication** is handled via [Clerk](https://clerk.com/). Set `CLERK_FRONTEND_API_URL` to your Clerk instance
> URL.

## Scripts (`mise.toml`)

| Command            | Description                                        |
| ------------------ | -------------------------------------------------- |
| `mise run`         | Generate OpenAPI, format, test, and lint (default) |
| `mise run setup`   | Clean & install all dependencies                   |
| `mise run back`    | Run backend server (`cargo run`)                   |
| `mise run front`   | Run frontend dev server (`pnpm start`)             |
| `mise run test`    | Run backend tests                                  |
| `mise run lint`    | Lint backend (Clippy + SQLx)                       |
| `mise run format`  | Format backend & frontend                          |
| `mise run openapi` | Generate OpenAPI specification                     |
| `mise run migrate` | Run database migrations                            |
| `mise run clean`   | Remove build artifacts                             |
| `mise run upgrade` | Upgrade backend & frontend dependencies            |

### Frontend Scripts (`frontend/`)

| Command      | Description                             |
| ------------ | --------------------------------------- |
| `pnpm start` | Serve Angular app (dev mode, port 4200) |
| `pnpm build` | Build for production                    |
| `pnpm test`  | Run unit tests (Vitest)                 |
| `pnpm lint`  | Check formatting (Prettier)             |

## Tests

- Backend: `mise run test`
- Frontend: `cd frontend && pnpm test`

Coverage reports are uploaded to [Codecov](https://codecov.io/gh/michaelcoll/arcane-exchange).

## Project Structure

```
.
├── src/ccpt/                          # Rust backend (hexagonal / clean architecture)
│   ├── main.rs                        # Entry point
│   ├── domain/                        # Domain models (Card, Price, User, LanguageCode…)
│   ├── application/                   # Use cases and service interfaces
│   └── infrastructure/
│       ├── adapter_in/                # HTTP controllers (Axum), auth extractor
│       └── adapter_out/               # DB repositories (SQLx), external callers
├── frontend/                          # Angular 21 frontend (pnpm + Tailwind CSS + Angular Material)
├── migrations/                        # SQLx SQL migration files
├── collection/                        # Bruno API collection for manual testing
├── example-files/                     # Example CSV and credentials files
├── Dockerfile                         # Backend Docker image
├── docker-compose.yml                 # Full-stack Compose setup
├── mise.toml                          # Task runner & toolchain configuration
└── Cargo.toml                         # Rust package manifest
```

### Architecture

The backend follows **Clean Architecture** with strict inward dependency flow: `Domain ← Application ← Infrastructure`.

- **Domain**: Pure business entities.
- **Application**: Use cases and repository traits.
- **Infrastructure**: Adapters (Axum, SQLx, External Callers).

Cards are identified by `CardId`: `set_code + collector_number + language_code + foil`.
