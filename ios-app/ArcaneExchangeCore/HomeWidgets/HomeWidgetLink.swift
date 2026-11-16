import Foundation

/// Where a link into the app takes the user: a tap on a home-screen widget, or Siri, Spotlight
/// and Shortcuts through the App Intents.
///
/// A scheme of its own, not the bundle id one: that one belongs to Clerk's OAuth callback
/// (`<bundle id>://callback`). Registered under `CFBundleURLTypes` in `project.yml`.
package enum HomeWidgetLink: Hashable {
    case collection
    case trades
    case trade(TradeDetailRoute)
    /// An impression: the app looks it up again by its key before showing it.
    case impression(ImpressionKey)

    package static let scheme = "arcane-exchange"

    /// `arcane-exchange://collection`, `…://trades`, `…://trades/<id>?partner=<username>`,
    /// `…://impression/<set>/<collector number>`.
    package var url: URL {
        var components = URLComponents()
        components.scheme = Self.scheme
        switch self {
        case .collection:
            components.host = "collection"
        case .trades:
            components.host = "trades"
        case let .trade(route):
            components.host = "trades"
            components.path = "/" + route.id
            components.queryItems = [URLQueryItem(name: "partner", value: route.partnerUsername)]
        case let .impression(key):
            components.host = "impression"
            components.percentEncodedPath = Self.path(of: key)
        }
        return components.url!
    }

    package init?(url: URL) {
        guard url.scheme == Self.scheme,
              let components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        else {
            return nil
        }
        let tradeID = components.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        switch components.host {
        case "collection":
            self = .collection
        case "trades" where tradeID.isEmpty:
            self = .trades
        case "trades":
            let partner = components.queryItems?.first { $0.name == "partner" }?.value ?? ""
            self = .trade(TradeDetailRoute(id: tradeID, partnerUsername: partner))
        case "impression":
            let segments = components.percentEncodedPath
                .split(separator: "/", omittingEmptySubsequences: false)
                .dropFirst()
                .compactMap(\.removingPercentEncoding)
            guard segments.count == 2, !segments[0].isEmpty, !segments[1].isEmpty else { return nil }
            let key = ImpressionKey(set: segments[0], number: segments[1])
            self = .impression(key)
        default:
            return nil
        }
    }

    /// `/<set>/<collector number>`, each segment encoded.
    private static func path(of key: ImpressionKey) -> String {
        "/" + [key.set, key.number].map(encodeSegment).joined(separator: "/")
    }

    /// A segment holds no `/`: a set code or a collector number never has one, but the path
    /// would split on it.
    private static func encodeSegment(_ segment: String) -> String {
        let allowed = CharacterSet.urlPathAllowed.subtracting(CharacterSet(charactersIn: "/"))
        return segment.addingPercentEncoding(withAllowedCharacters: allowed) ?? segment
    }
}
