---
status: accepted
date: 2026-11-12
---

# Sur iOS, tout ce qui n'est pas un écran vit dans un framework partagé

Les tests unitaires iOS font `@testable import ArcaneExchange` : xcodebuild installe et lance l'app
hôte avant le premier test (~70 s en CI pour moins d'une seconde de tests), et l'extension des
widgets recompile à la main une liste de sources de l'app. On sort donc de l'app tout ce qui n'est
pas un écran dans une **cible framework XcodeGen nommée `ArcaneCore`** (dossier `ios-app/ArcaneCore/`,
`import ArcaneCore`). L'app l'embarque, l'extension le lie, la cible de tests le lie seul : elle tourne
sans app hôte. Règle de rangement : **écran → app, le reste → framework**.

## Frontière

- **Dans `ArcaneCore`** : modèles, ViewModels, `Networking`, `Config`, `Support` sans dépendance UI
  lourde, `HomeWidgets/` (vues des widgets comprises, y compris `HomeWidgetReload`), l'asset catalog
  des couleurs et le shader du foil. `HomeWidgetReload` rejoint le framework : WidgetKit y est déjà
  importé par les vues des widgets, et `ImportViewModel` l'appelle.
- **Dans l'app** : les écrans (`*View`, sheets, rows), `ArcaneExchangeApp`, `ContentView`,
  `MissingClerkKeyView`, `Settings.bundle`, l'icône, la police Keyrune (`UIAppFonts` ; le framework
  ne porte que les tables de codepoints) et tout fichier qui importe `ClerkKitUI` ou `NukeUI`.
  `PlayerAvatarStore` est séparé de la vue `PlayerAvatar` : le store va dans le framework, la vue reste.
- **Dépendances du framework** : `APIClient`, `ClerkKit` (pas `ClerkKitUI`), `OpenAPIRuntime`,
  `OpenAPIURLSession`, `HTTPTypes`, `Nuke` (pas `NukeUI`). Il ne dépend ni de `ClerkKitUI` ni de
  `NukeUI`, pour que l'extension ne les embarque pas.
- **Bundle** : `Palette` résout ses couleurs dans le bundle du framework. `AppConfig` et
  `SharedContainer` lisent l'Info.plist du processus (app ou extension) ; sans hôte, `Bundle.main` est
  le runner de tests, donc leurs tests injectent la source (bundle ou dictionnaire).

## Visibilité : `package`, pas `public`

Les types vus de l'app, de l'extension et des tests prennent le niveau d'accès **`package`**, et les
trois cibles plus le framework partagent `SWIFT_PACKAGE_NAME = arcane-exchange`, posé une fois dans
`Config/Shared.xcconfig` (qui s'applique à toutes les cibles). Les tests gardent
`@testable import ArcaneCore` pour les types `internal`.

Vérifié sur un projet XcodeGen jetable (Xcode 27.2, XcodeGen 2.46) : un framework, une app, une
extension et une cible de tests sans hôte, avec `SWIFT_PACKAGE_NAME` dans les réglages de base.
L'app et l'extension voient les symboles `package`, le test lit aussi un type `internal`, le tout
passe. Sans `SWIFT_PACKAGE_NAME`, la compilation échoue (« the package access level used on 'X'
requires a package name »).

`package` plutôt que `public` : le framework n'a que des clients dans ce dépôt et n'est pas une
bibliothèque ; `public` forcerait à rendre publics initialiseurs, membres et conformances, et un
`public` oublié se découvre en compilant l'autre cible. `package` coûte le même nombre de mots-clés
mais garde l'API hors de portée de tout module extérieur au dépôt.

## Options écartées

- **Nom du framework** : `Core` seul (le nom d'attente de la spec) est trop générique pour un module
  importé partout, et colle au préfixe des frameworks Apple (`CoreData`, `CoreGraphics`) ; `Shared`
  décrit qui l'utilise, pas ce qu'il contient ; `ArcaneExchange` est déjà le nom du module de l'app.
  `ArcaneCore` garde le préfixe du produit, reste court et dit « le cœur sans les écrans ».
- **`HomeWidgetReload` dans l'app** : `ImportViewModel`, qui vit dans le framework, l'appelle après un
  import ; le laisser dans l'app obligerait à lui injecter un rappel (ou un protocole) pour six lignes
  de code, alors que WidgetKit est déjà lié par les vues des widgets du framework. Sa place dans le
  framework ne coûte aucune dépendance de plus.

## Conséquences

- Un nouveau type utilisé par les widgets s'ajoute au framework, sans ligne `sources` à tenir dans
  `project.yml` : l'extension ne liste plus de sources de l'app.
- Les entitlements (App Group, groupe Keychain) restent sur l'app et l'extension seules.
- Pas de paquet Swift local : l'asset catalog, le `.metal` et UIKit lient de toute façon le module au
  simulateur, et une cible XcodeGen ne change ni mise ni le cache CI.
- Un seul module : pas de découpage par domaine ni de module Design System séparé.
