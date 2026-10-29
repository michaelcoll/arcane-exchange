# Coding Standards

The rules a diff is checked against: each one is specific to this repository and enforced by no tool. Whatever
rustfmt, clippy, ESLint, Prettier, SwiftLint or SwiftFormat already reports is left to them.

## Cross-cutting

- **Language**: write code, comments and commit messages in English, and user-facing strings in French: the product
  is French-only, the codebase is not.
- **Naming**: name identifiers, DTOs and UI labels with the terms of [`GLOSSARY.md`](GLOSSARY.md), never with one it
  lists under _Avoid_ (a single occurrence in a diff is a deviation): one concept keeps one name from the database to
  the screen.
- **Glossary term form**: write an identifier with the glossary's bold term when it is English (`Trade`, `CardId`) and
  with its English translation when it is French (`Rareté` → `Rarity`): the Language rule keeps French out of code.

## Backend

- **Service layout**: put each use case in `application/service/<resource>/<action>.rs`, where `<resource>` is the
  `adapter_in/` folder whose controller calls it (or triggers it, for a use case with no route) and `<action>` is the
  use case trait name without the suffix nor what the folder already says, in snake_case (`trade/accept.rs`,
  `trade_settings/get_visibility.rs`; `get` for a trait with no verb, `stats/get.rs`): one use case per file,
  one folder per resource even with a single use case, the service struct keeps its full name. Code shared by the use
  cases of a resource goes in the parent module `service/<resource>.rs`, its shared test fixtures in a test module of
  that parent; `service/` holds nothing but use cases (ADR 0019).
- **Service tests**: test a service in a `mod tests` at the bottom of its own file, against the `mockall` mocks of its
  ports (`#[cfg_attr(test, automock)]` on the trait): the test needs no database and no HTTP.
- **Controller tests**: test a controller in the `tests.rs` of its folder, driving the router with `oneshot` on an
  `AppState::for_testing()` whose use cases under test are overridden: the request goes through routing and
  extractors, which a direct handler call skips.
- **Repository tests**: test a repository adapter with `#[sqlx::test]`, against a real database: its SQL is the thing
  under test.
- **No panic outside tests**: return an `InfraError` instead of `unwrap` / `expect` on a value that came from outside
  (database, HTTP), even one a constraint guarantees: corrupt data is a 500, neither a panic nor a caller mistake
  (4xx) — as in `backend/src/ae/infrastructure/adapter_out/repository/entities.rs` (1ac6edf).
- **TypeScript export**: export a DTO with `#[ts(export)]` alone, never `export_to`: ts-rs names the file after the TS
  type name, which already follows `#[serde(rename = "X")]` (`CardOfferResponse` → `CardOffer.ts`).
- **`Option<Struct>` DTO field**: annotate it `#[schema(value_type = TheStruct, required = false)]`: otherwise utoipa
  emits a schema the Swift generator drops, and the field silently disappears from the iOS client.

## Web

Token names and values: [design-system.instructions.md](.agents/design-system.instructions.md).

- **Styling**: compose Tailwind classes in the template, with no `<style>` block in a `.vue` component except
  `<style scoped>` for what utilities cannot express — complex `@keyframes`, layered gradients and pseudo-elements (the
  foil effect in `app/components/MtgCard.vue`): visual composition is utility-first, with no dedicated CSS classes.
- **Accents**: color an accent with the semantic Tailwind colors `primary`, `secondary` and `rarity` (`text-primary`,
  `bg-secondary/10`, `text-rarity-rare`): they follow the theme without a `dark:` pair.
- **Other tokens**: apply them with the arbitrary-value syntax (`bg-[var(--surface)]`, `text-[var(--ink-2)]`,
  `rounded-[var(--r-lg)]`): they have no Tailwind utility (there is no `bg-surface`).
- **Typography**: pick the font by role — `font-display` for titles, `font-mono` for numbers, prices and labels,
  `font-sans` for everything else: the three families each carry one meaning.
- **Theme**: style the dark theme through the `.dark` class on `<html>` (Tailwind `darkMode: 'class'`), never a
  `data-theme` attribute: that is the mockup's mechanism, and nothing in `frontend/` sets it.
- **Icons**: render an icon with `<Icon name="lucide:…" :size="…" />` only, never a hand-rolled SVG icon component or
  an inline icon path: one icon set, loaded by `@nuxt/icon`.
- **Brand logos**: render a brand logo (Discord, GitHub…) with `simple-icons:…`: Lucide ships no brand icon
  (`lucide:github` is deprecated).
- **API errors**: read a caught API error with `readApiError` (`app/utils/api-error.ts`) and show its `message`, never
  the raw error response (`e.data.error`, `e.statusCode`, `e.response.status`) nor an `instanceof` test: the API's
  message is technical and in English (ADR 0018), and `useAsyncData` rewraps the error, keeping only its status and body.
- **Resource services**: make each function of a `use<Resource>Service` composable return a typed promise
  (`Promise<T>`, `T` from `app/bindings/`) through `useApi`, and let its caller — page or component — wrap it in
  `useAsyncData` under a key it picks; never `useAsyncData`, a cache key or a `MaybeRefOrGetter` in a service: the
  caller alone knows its key and its reactivity. Put non-trivial reactive behaviour in a dedicated composable, as
  `app/composables/useSearchSets.ts` does.
- **Test suffix**: name a test file `*.spec.ts`, never `*.test.ts`: one suffix for the whole suite.
- **Test location**: put a test that runs in plain Node beside the file it tests, a test that needs the Nuxt runtime
  directly in `test/nuxt/`, and an end-to-end test directly in `test/e2e/`: each Vitest project (`unit`, `nuxt`,
  `e2e`) includes only its own location, and the two `test/` globs are not recursive.

## iOS

- **Tests**: write tests with Swift Testing, never XCTest: the whole suite is `import Testing`.
- **Design**: never port the mockup's HTML/CSS structure into SwiftUI: the mockup is a reference (`maquette-ios`
  skill) adapted to iOS idioms, with its tokens in
  [design-system.instructions.md](.agents/design-system.instructions.md).
- **Colors**: name a color by role, as on the web — `Palette.primary` / `.primaryInk` / `.onPrimary` and their
  `secondary` counterparts (`Support/Palette.swift`), `Palette.down` for a value drop, `RarityColor` for the five
  rarities. Never `Color.accentColor`, SwiftUI `.cyan` / `.purple` for an accent, or SwiftUI `.red` for a value drop:
  they bypass the role and the web's values.
- **Palette members**: keep the accents in the `Palette` namespace, never as `Primary` / `Secondary` `ShapeStyle`
  members: they collide with SwiftUI's `.primary` / `.secondary` label styles.
- **Color values**: change a color asset together with its light/dark token in `frontend/app/assets/css/main.css`:
  the two are the same values, pinned by `PaletteTests`.
- **Images**: load an image with `LazyImage` (NukeUI) through `ArtworkPipeline`, never `AsyncImage`: it has no
  decoded-image cache and re-downloads on scroll-back.
- **Prices**: format a price with `Price.euros(cents:)`: it is pinned to `fr_FR` whatever the device locale.
- **View models**: declare a view model `@Observable`, never `ObservableObject`: views own them with `@State`.
- **Pure logic**: put pure logic in a type of its own (`LoginCopy`, `AccountIdentity`), never on a `View`, not even
  as a `static func`: a `View` is `@MainActor`, and swift-testing crashes calling into it.
- **API errors**: read only the success of a call through the generated accessor (`.ok`, `.noContent`…) and catch a
  failure with `APIError.read` (`Networking/APIError.swift`), showing its `message` and testing its `code`, never a
  generated error case, a status or a `switch` on the `Output`: the client middleware raises every failure as an
  `APIError` (ADR 0020), whose messages mirror the web's.

## Complementary sources

These files keep their own rules; a review loads them too.

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — backend layer rules: direction of dependencies, injection as
  `Arc<dyn Trait>` wired in `create_infra`, services grouped by HTTP resource with one file per use case,
  configuration read once.
- [`.agents/design-system.instructions.md`](.agents/design-system.instructions.md) — token reference and component
  inventory.
- [`.agents/database-schema.instructions.md`](.agents/database-schema.instructions.md) and
  [`.agents/trade-workflow.instructions.md`](.agents/trade-workflow.instructions.md) — data and workflow invariants.
