import Foundation

enum AppConfig {
    /// `UserDefaults` key backing the "URL de l'API" row of the app's pane in the iOS
    /// Settings app (`Settings.bundle/Root.plist`). The Settings app writes it, we read it.
    ///
    /// Keep it in sync with the `Key` entry of that plist — there is no shared symbol
    /// between a Settings bundle and the app binary.
    static let apiBaseURLDefaultsKey = "api_base_url"

    /// Base URL the API client talks to, i.e. the server root **including** the `/api/v1`
    /// prefix (same contract as `API_BASE_URL` in the xcconfigs).
    ///
    /// Reads the Settings override first, then the value baked in at build time. A blank or
    /// malformed override falls back to the built-in value rather than crashing the app, so a
    /// typo in Settings stays recoverable from the app itself.
    static var apiBaseURL: URL {
        let override = UserDefaults.standard.string(forKey: apiBaseURLDefaultsKey)
        return override.flatMap(parseBaseURL) ?? bundledAPIBaseURL
    }

    /// The `API_BASE_URL` of the build configuration, before any Settings override.
    /// Also what the Settings pane shows as its placeholder default.
    static var bundledAPIBaseURL: URL {
        guard let value = Bundle.main.object(forInfoDictionaryKey: "API_BASE_URL") as? String,
              let url = parseBaseURL(value)
        else {
            fatalError("Missing or invalid API_BASE_URL in Info.plist")
        }
        return url
    }

    /// Clerk instance the app authenticates against, baked in from `CLERK_PUBLISHABLE_KEY`.
    ///
    /// Blank on a clone without `Config/Local.xcconfig` — `Shared.xcconfig` declares an empty
    /// default so the app still builds and runs. Sign-in is what fails then; copy
    /// `Config/Local.xcconfig.example` and fill the key in.
    static var clerkPublishableKey: String {
        guard let value = Bundle.main.object(forInfoDictionaryKey: "CLERK_PUBLISHABLE_KEY") as? String else {
            fatalError("Missing CLERK_PUBLISHABLE_KEY in Info.plist")
        }
        return value
    }

    /// Copies the build-time base URL into `UserDefaults` on first launch.
    ///
    /// `UserDefaults.register(defaults:)` would not do: it only feeds reads inside the process,
    /// so the Settings app would show an empty field while the app talks to the built-in URL.
    /// Writing the value makes the Settings pane display what is actually in use, and keeps the
    /// xcconfig the single place the default is declared (no `DefaultValue` in `Root.plist`).
    static func seedSettingsDefaults() {
        let defaults = UserDefaults.standard
        guard defaults.string(forKey: apiBaseURLDefaultsKey) == nil else { return }
        defaults.set(bundledAPIBaseURL.absoluteString, forKey: apiBaseURLDefaultsKey)
    }

    /// Accepts an absolute `http(s)` URL only: `URL(string:)` alone happily parses "localhost"
    /// or "" into something the transport would then fail on at request time.
    static func parseBaseURL(_ raw: String) -> URL? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let url = URL(string: trimmed),
              let scheme = url.scheme?.lowercased(),
              scheme == "http" || scheme == "https",
              url.host?.isEmpty == false
        else {
            return nil
        }
        return url
    }
}
