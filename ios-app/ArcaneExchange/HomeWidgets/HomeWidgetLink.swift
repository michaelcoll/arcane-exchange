import Foundation

/// Where a tap on a home-screen widget takes the user.
///
/// A scheme of its own, not the bundle id one: that one belongs to Clerk's OAuth callback
/// (`<bundle id>://callback`). Registered under `CFBundleURLTypes` in `project.yml`.
enum HomeWidgetLink: Hashable {
    case collection
    case trades
    case trade(TradeDetailRoute)

    static let scheme = "arcane-exchange"

    /// `arcane-exchange://collection`, `…://trades`, `…://trades/<id>?partner=<username>`.
    var url: URL {
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
        }
        return components.url!
    }

    init?(url: URL) {
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
        default:
            return nil
        }
    }
}
