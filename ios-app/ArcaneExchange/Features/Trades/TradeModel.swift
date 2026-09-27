import APIClient
import SwiftUI

/// Short names for the generated schema types the Échanges screens work with.
typealias TradeSummary = Components.Schemas.TradeSummaryResponse
typealias TradeDetail = Components.Schemas.TradeDetailResponse
typealias TradeCard = Components.Schemas.TradeCardResponse
typealias TradeStatusParam = Components.Schemas.TradeStatusParam

/// The trade state machine, mirroring the backend's `TradeStatus` (`src/ae/domain/trade.rs`)
/// and the web client's `utils/trade.ts`. See `.agents/trade-workflow.instructions.md`.
///
/// The API types `status` as a plain string, so this narrows it, falling back to `.pending`
/// exactly like `toTradeStatus` on the web.
enum TradeStatus: String, CaseIterable, Hashable {
    case pending = "PENDING"
    case oneAccepted = "ONE_ACCEPTED"
    case fullyAccepted = "FULLY_ACCEPTED"
    case completed = "COMPLETED"
    case closed = "CLOSED"
    case abandoned = "ABANDONED"

    init(apiValue: String) {
        self = TradeStatus(rawValue: apiValue) ?? .pending
    }

    var label: String {
        switch self {
        case .pending: "En négociation"
        case .oneAccepted: "1 acceptation"
        case .fullyAccepted: "Verrouillé"
        case .completed: "Échange réalisé"
        case .closed: "Clôturé"
        case .abandoned: "Abandonné"
        }
    }

    /// Same roles as the web's `TRADE_STATUS_META`: a trade moving forward is primary, a locked
    /// one — cards reserved for the other player — secondary.
    var tint: Color {
        switch self {
        case .pending, .oneAccepted: Palette.primary
        case .fullyAccepted: Palette.secondary
        case .completed, .closed: .green
        case .abandoned: .red
        }
    }

    /// Text and glyphs sitting on a `tint`-tinted fill (the status pill).
    var ink: Color {
        switch self {
        case .pending, .oneAccepted: Palette.primaryInk
        case .fullyAccepted: Palette.secondaryInk
        case .completed, .closed, .abandoned: tint
        }
    }

    /// Glyph colour for a disc filled with `tint`: the role's `on-*` color, near-black on the
    /// bright green and white on the red.
    var onTint: Color {
        switch self {
        case .pending, .oneAccepted: Palette.onPrimary
        case .fullyAccepted: Palette.onSecondary
        case .completed, .closed: Color(red: 0.02, green: 0.1, blue: 0.11)
        case .abandoned: .white
        }
    }

    var symbol: String {
        switch self {
        case .pending: "arrow.left.arrow.right"
        case .oneAccepted, .fullyAccepted: "lock.fill"
        case .completed, .closed: "checkmark"
        case .abandoned: "xmark"
        }
    }

    /// Cards can still be added or removed.
    var isEditable: Bool {
        self == .pending || self == .oneAccepted
    }

    /// Cards on both sides are locked out of any other trade.
    var isReserved: Bool {
        self == .oneAccepted || self == .fullyAccepted
    }

    /// Still live — what the "En cours" segment of the list shows.
    var isOngoing: Bool {
        self != .closed && self != .abandoned
    }

    /// The nominal path, `abandoned` excluded: it is a dead end, not a step.
    static let lifecycle: [TradeStatus] = [.pending, .oneAccepted, .fullyAccepted, .completed, .closed]

    /// Position in `lifecycle`, `nil` for `.abandoned`.
    var lifecycleIndex: Int? {
        Self.lifecycle.firstIndex(of: self)
    }
}

/// Which half of the trade a rail shows, from the user's point of view — it decides the rail's
/// colors: « Je donne » stays neutral, « Je reçois » and the partner's name are secondary.
enum TradeSide {
    /// My own cards, leaving.
    case give
    /// The partner's cards, coming to me.
    case receive

    /// The rail's total and each card's value.
    var valueTint: Color {
        switch self {
        case .give: .primary
        case .receive: Palette.secondary
        }
    }

    /// The owner's handle in the rail header; `nil` keeps the header's neutral style.
    var ownerTint: Color? {
        switch self {
        case .give: nil
        case .receive: Palette.secondary
        }
    }
}

/// What each side is worth and who owes whom, in cents.
///
/// `diff > 0` means the cards coming to the user are worth more than the ones leaving, so the
/// user pays the difference — settled in person, outside the platform.
struct TradeBalance: Equatable {
    /// Below 3 €, the web client calls the trade even rather than quoting a token amount.
    private static let evenThreshold = 300

    let giveCents: Int
    let getCents: Int

    init(give: [TradeCard], get: [TradeCard]) {
        giveCents = Self.total(of: give)
        getCents = Self.total(of: get)
    }

    /// Trend price × quantity, a missing price counting as zero — same rule as `tradeCardValue`.
    static func total(of cards: [TradeCard]) -> Int {
        cards.reduce(0) { $0 + Int($1.price_guide?.trend ?? 0) * Int($1.quantity) }
    }

    var diffCents: Int {
        getCents - giveCents
    }

    var isEven: Bool {
        abs(diffCents) < Self.evenThreshold
    }

    /// "Équilibré" / "Tu dois 21 €" / "On te doit 4 €".
    var verdict: String {
        if isEven {
            return "Équilibré"
        }
        let amount = Price.euros(cents: abs(diffCents))
        return diffCents > 0 ? "Tu dois \(amount)" : "On te doit \(amount)"
    }

    /// The settlement half of the accept button — "payer 21 €", "recevoir 4 €", `nil` if even.
    var settlementLabel: String? {
        guard !isEven else { return nil }
        let amount = Price.euros(cents: abs(diffCents))
        return diffCents > 0 ? "payer \(amount)" : "recevoir \(amount)"
    }

    /// Share of the total value sitting on the "je reçois" side, for the balance bar.
    var receivedShare: Double {
        let total = giveCents + getCents
        return total == 0 ? 0.5 : Double(getCents) / Double(total)
    }
}

enum TradesCopy {
    static func cardCount(_ count: Int) -> String {
        count > 1 ? "\(count) cartes" : "\(count) carte"
    }

    /// "il y a 30 min" — the list row's timestamp, from the summary's RFC 3339 string.
    static func relativeDate(from iso: String) -> String {
        guard let date = TradeTimestamp.date(from: iso) else { return "" }
        return date.formatted(.relative(presentation: .named))
    }
}

/// Parses the `updated_at` the API sends.
///
/// The backend hands over `chrono`'s `to_rfc3339`, which emits up to nine fractional digits and
/// a `+00:00` offset. `ISO8601DateFormatter`'s `.withFractionalSeconds` is picky about how many
/// digits it accepts, and a "il y a 30 min" label needs none of them, so they are dropped
/// before parsing rather than guessed at.
enum TradeTimestamp {
    /// Safe to share: `ISO8601DateFormatter` is thread-safe once configured, and this one is
    /// never mutated again.
    private nonisolated(unsafe) static let formatter: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        return formatter
    }()

    static func date(from iso: String) -> Date? {
        formatter.date(from: iso.replacing(/\.\d+/, with: ""))
    }
}

extension TradeCard {
    /// Stable identity for a line: the API has no id, the four fields are the trade's own key.
    var lineID: String {
        "\(set_code)-\(collector_number)-\(language_code)-\(foil)"
    }
}

/// Pushed onto whichever stack opens a trade — the Échanges list today, the owners screen
/// when starting one from a card.
struct TradeDetailRoute: Hashable {
    let id: String
    let partnerUsername: String
}
