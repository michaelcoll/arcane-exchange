# GitHub Actions CI/CD Guide

## Pipelines

### 1. Backend CI (`lint-test-backend.yml`) — triggered on every push

- **lint**: `rustfmt` + `clippy` (runs directly on `ubuntu-latest`)
- **test**: `cargo llvm-cov nextest` (in a `rust:1-bullseye` container with a PostgreSQL service, unpinned/`latest`
  tag) with coverage uploaded to Codecov
- **build-offline**: `SQLX_OFFLINE=true cargo build` to validate SQLX metadata
- **check-openapi**: regenerates `doc/openapi.yml` (`cargo run --bin generate-openapi`) and checks it's up to date

### 2. Frontend CI (`lint-test-frontend.yml`) — triggered on every push

- **format**: Prettier (`pnpm format`, i.e. `prettier --check`)
- **typecheck**: `pnpm lint` (`nuxi typecheck`)
- **build-production**: `pnpm build` (plain `nuxt build`)
- **build-dev**: `pnpm build --configuration development` — `--configuration` is an Angular-CLI-era leftover that
  `nuxi build` doesn't recognize; nuxi silently takes `development` as its positional `ROOTDIR` instead, so the job
  builds into a stray `frontend-vue/development/` folder rather than actually validating a dev config. It currently
  passes (Nuxt just finds the config one directory up) but is effectively a no-op duplicate of build-production —
  worth fixing or removing if you touch this workflow.
- there is no Vitest job in this workflow; `pnpm test` only runs locally / via `mise run test-frontend`

### 3. iOS CI (`lint-test-ios.yml`) — triggered on every push

- Runs on `macos-26` (pinned, not `macos-latest`), unlike backend/frontend CI which run on `ubuntu-latest` — a
  native Xcode build requires macOS. The image must ship Xcode 26 (default 26.6): `clerk-ios` >= 1.2.0 declares
  `swift-tools-version: 6.2`, which the Xcode 16.4 of `macos-15` cannot resolve ("incompatible tools version")
- **lint** (one job): SwiftFormat (`swiftformat --lint`) then SwiftLint (`swiftlint lint --strict`)
- **test** (one job): `xcodebuild test` against an iPhone 17 simulator — no separate `build` job, since `test`
  already builds the app and its test bundle first; a prior `build` job duplicated that work (twice: Debug's
  default `ARCHS` compiled both `arm64` and `x86_64` for the generic-destination build, vs. `arm64` only for the
  concrete simulator destination `test` uses) for no extra coverage
  - Two things run in parallel with the rest of that job to hide fixed costs rather than pay them serially:
    a background `simctl boot` right after checkout overlaps the simulator's ~3 min cold boot with
    `xcodegen generate` + the build itself (`simctl boot` on an already-booting device is a harmless no-op, so
    `xcodebuild test` booting it again costs nothing); an `actions/cache` step keyed on `project.yml` +
    `APIClient/Package.swift` (passed to `xcodebuild` as `-clonedSourcePackagesDirPath`) avoids re-fetching all
    Swift package checkouts from GitHub on every run — SwiftPM only re-resolves on a cache miss, so a dependency
    bump still ships as soon as one of those two files changes
  - Piped through `xcbeautify` for readable output — raw `xcodebuild` logs thousands of `WriteAuxiliaryFile`/
    `builtin-copy` lines per run
- Every job independently runs `xcodegen generate` first (project isn't committed, see
  [mise.instructions.md](mise.instructions.md)); tools installed via `brew` rather than a full `mise` bootstrap, to
  stay minimal like `lint-test-frontend.yml`'s direct `pnpm/action-setup` usage
- No signing/provisioning/TestFlight — simulator build/test only

### 4. Build & Push (`build-push.yml`) — triggered on push to `main` or on release

- Builds and publishes backend and frontend Docker images to **GHCR**
- On release: semver bump + sourcemap upload to Sentry
- Platform: `linux/amd64` only

### 5. PR Automation (`automerge.yml`, `pr-label.yml`, `clean-cache.yml`)

- **automerge**: dependabot patch/minor auto-merged
- **pr-label**: conventional labels (fix/feat/chore/ci) from the PR title
- **clean-cache**: removes the GitHub runner cache when a PR is closed

## Local Configuration

- **mise** (`mise.toml`): toolchain and task management. `mise run <task>` works for every task; a few (e.g. `back`,
  `front`) also work as a bare `mise <task>` shorthand, but there's no combined `test` or `lint` task — use
  `test-backend`/`test-frontend` and `lint-backend`/`lint-frontend` (see
  [mise.instructions.md](mise.instructions.md)).
- **pnpm**: CI reads the version from `frontend-vue/package.json`'s `packageManager` field (`pnpm@11.21.0`), so it
  always matches `mise.toml`'s `pnpm = "11"` — no manual version sync needed.

## Docker

- **Backend**: multi-stage (rust → distroless nonroot, port 8080)
- **Frontend**: multi-stage (node → nginx-alpine, SPA routing)
- **docker-compose**: postgres 18, `ae` backend, frontend nginx (port 9797)
