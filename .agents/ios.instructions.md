# iOS App Development Guide (Swift/SwiftUI)

The native client lives in `ios-app/`. It talks to the same Rust backend as `frontend-vue/`, through a client
generated from the very same `doc/openapi.yml`.

## Project Generation (CRITICAL)

- **`ios-app/project.yml` is the source of truth**, not the Xcode project: `ArcaneExchange.xcodeproj/` is generated
  by XcodeGen and git-ignored. **Never edit `project.pbxproj`** — the next `xcodegen generate` throws the change
  away. Edit `project.yml`, then `mise run build-ios` (which depends on the hidden `generate-ios` task).
- **`ArcaneExchange/Info.plist` is generated too**, from the `info.properties` block of `project.yml`. Add plist keys
  there, not in the file.
- **Build settings**: shared/global values go in `Config/*.xcconfig` (`Shared` + `Dev`/`Release`, with the
  git-ignored `Local.xcconfig` for per-developer signing and Clerk key). A target-level setting in `project.yml`
  wins over an xcconfig, so per-target values belong in `settings.base` (e.g. `TARGETED_DEVICE_FAMILY: "1"`).
- **Baseline**: iOS 18 deployment target, Swift 6 (strict concurrency), **iPhone only** (`TARGETED_DEVICE_FAMILY`
  = `"1"`). Adding iPad back means declaring the four interface orientations in `info.properties`.
- `//` starts a comment in an xcconfig: a URL must be written `http:/$()/host` (see `Dev.xcconfig`).
- **Xcode 26 required** (locally and in CI): `clerk-ios` >= 1.2.0 ships a `swift-tools-version: 6.2` manifest that
  Xcode 16 refuses to resolve ("incompatible tools version").

## Structure

- `ArcaneExchange/Features/<Feature>/` — one folder per tab/screen (`View`, `ViewModel`, cells, sheets).
- `ArcaneExchange/Networking/` — `APIClientProvider`, `ClerkAuthMiddleware`.
- `ArcaneExchange/Config/` — `AppConfig` (build-time + Settings-app configuration).
- `ArcaneExchange/Support/` — cross-feature helpers (`Price`, `CardArtwork`, `ArtworkPipeline`).
- `ArcaneExchange/Settings.bundle/` — the app's pane in the iOS Settings app (folder reference, copied verbatim).
- `ArcaneExchangeTests/` — tests, **Swift Testing** (`import Testing`, `@Test`, `#expect`), not XCTest.

## API Client

- `APIClient/` is a local SwiftPM package whose sources are **generated at build time** by
  `swift-openapi-generator` from `APIClient/Sources/APIClient/openapi.yaml`, a symlink to `doc/openapi.yml`.
  Nothing generated is committed; `APIClient.swift` is intentionally empty.
- **An API change starts in the backend + `doc/openapi.yml`**, never by hand-writing Swift models.
- A backend DTO field typed `Option<Struct>` must carry `#[schema(value_type = TheStruct, required = false)]`,
  otherwise utoipa emits a schema the generator drops — the field silently disappears from the Swift client.
- Use `APIClientProvider.shared` (rebuilt on each access on purpose: the base URL is editable at runtime).
  `ClerkAuthMiddleware` adds `Authorization: Bearer <token>` from `Clerk.shared.auth.getToken()` on every request,
  mirroring `frontend-vue/app/composables/useApi.ts`.
- Status codes with no typed schema in the OpenAPI surface as `.undocumented(statusCode:_)` on each operation's
  `Output` enum. Switch per operation and map to `APIClientError.unauthorized` / `.undocumented(statusCode:)`; the
  view model then turns that into a user-facing case (see `CollectionViewModel.LoadError`).

## SwiftUI Conventions

- **View model**: `@MainActor @Observable final class …ViewModel`, held by the view as `@State private var model`.
  Loading/error/pagination state is `private(set)` on the model; the view stays declarative.
- **Async**: `.task { }` / `.task(id: model.filters) { }` for loads and reloads, `.refreshable` for pull-to-refresh.
  No Combine, no completion handlers.
- **Native first**: `TabView`/`Tab` (the search tab uses `role: .search`), `NavigationStack`, sheets, system
  materials and `ProgressView`. Do not port the mockup's HTML/CSS structure into SwiftUI.
- **Design reference**: the iOS mockup, via the `maquette-ios` skill; underlying tokens in
  [design-system.instructions.md](design-system.instructions.md), adapted to iOS idioms rather than copied.
- **Images**: Nuke / NukeUI (`LazyImage`) through `ArtworkPipeline` (aggressive `DataCache` + prefetching around
  pagination). Do not use `AsyncImage` — it has no decoded-image cache and re-downloads on scroll-back.
- **Prices**: `Price.euros(cents:)`, pinned to `fr_FR` regardless of device locale; deal thresholds in `CardDeal`
  mirror the web client's `Card/Cell.vue` (±3 % is noise).
- **Language**: UI strings in French (the product is French), code and comments in English, like the rest of the
  repo.

## Runtime Configuration

- `AppConfig` reads `API_BASE_URL` and `CLERK_PUBLISHABLE_KEY` from `Info.plist` (fed by the xcconfigs at build
  time). The base URL is overridable at runtime from Settings ▸ Arcane Exchange (`UserDefaults` key
  `api_base_url`, declared in `Settings.bundle/Root.plist` — keep both in sync, they share no symbol).
- Debug builds point at `http://localhost:8080/api/v1`; the `CLERK_PUBLISHABLE_KEY` in `Local.xcconfig` must match
  the Clerk instance that backend validates against. See
  [authentication.instructions.md](authentication.instructions.md).

## Commands & CI

| Action     | Command               | Alias         |
| ---------- | --------------------- | ------------- |
| **Build**  | `mise run build-ios`  | `mise run bi` |
| **Test**   | `mise run test-ios`   | —             |
| **Lint**   | `mise run lint-ios`   | —             |
| **Format** | `mise run format-ios` | —             |

These are standalone tasks, deliberately outside `checks`/`format`/`setup` (see
[mise.instructions.md](mise.instructions.md)). `.github/workflows/lint-test-ios.yml` runs `swiftformat --lint`,
`swiftlint --strict`, build and tests on every push — run format + lint locally before pushing, `--strict` fails on
warnings.
