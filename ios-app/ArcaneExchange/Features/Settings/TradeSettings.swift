import APIClient

/// Short names for the schema types the Réglages screen works with.
typealias CollectionVisibility = Components.Schemas.CollectionVisibilityParam
typealias BinderInfo = Components.Schemas.BinderInfoResponse
typealias RarityFilter = Components.Schemas.RarityFilterResponse

extension CollectionVisibility {
    /// The mockup's `VIS_LBL`, same wording as the web client's `visHelp`.
    var label: String {
        switch self {
        case ._public: "Publique"
        case .trade: "Échangeable"
        case ._private: "Privée"
        }
    }

    var detail: String {
        switch self {
        case ._public: "Tout le monde voit toute ta collection, y compris les cartes que tes règles ne proposent pas."
        case .trade: "Seules les cartes des classeurs sélectionnés, après filtres de rareté, sont visibles."
        case ._private: "Personne ne voit ta collection. Tu n'apparais dans aucune recherche."
        }
    }

    /// Listed the way the mockup orders them, from the most open to the most closed.
    static let ordered: [CollectionVisibility] = [._public, .trade, ._private]
}

enum TradeRules {
    /// Mirrors the backend's `MAX_KEPT_COPIES` (a higher value is rejected with a 400) and the
    /// web client's `utils/trade-rules.ts`.
    static let maxKeptCopies: Int32 = 4
}

/// How the copies owned inside the selected binders split up, from the rarity rules — the
/// mockup's `TradeRatio` band.
///
/// A plain struct rather than a `View` member: `View` is `@MainActor`, and this pure arithmetic
/// is exercised from tests.
struct TradeRatio: Equatable {
    let proposed: Int
    let kept: Int
    let excluded: Int

    init(rarities: [RarityFilter]) {
        proposed = rarities.reduce(0) { $0 + ($1.is_open ? Int($1.proposed) : 0) }
        kept = rarities.reduce(0) { $0 + ($1.is_open ? Int($1.copies) - Int($1.proposed) : 0) }
        excluded = rarities.reduce(0) { $0 + ($1.is_open ? 0 : Int($1.copies)) }
    }

    var total: Int {
        proposed + kept + excluded
    }

    /// Share of `total`, in `0...1`. Zero when nothing is owned, so the band collapses instead
    /// of dividing by zero.
    func share(_ count: Int) -> Double {
        total == 0 ? 0 : Double(count) / Double(total)
    }

    /// "38 % de la collection est proposée", as the mockup captions the band.
    var summary: String {
        "\(Int((share(proposed) * 100).rounded())) % de la collection est proposée"
    }
}

enum AccountCopy {
    /// "2 sur 5" — how many binders are opened to trade, out of those the last import found.
    static func binderSelection(selected: Int, total: Int) -> String {
        "\(selected) sur \(total)"
    }

    /// "1 240 exemplaires", grouped the French way like the rest of the UI.
    static func copies(_ count: Int) -> String {
        let formatted = count.formatted(.number.locale(.init(identifier: "fr_FR")))
        return count > 1 ? "\(formatted) exemplaires" : "\(formatted) exemplaire"
    }

    /// "1 240 cartes" — a binder's size, from `GET /collection/stats`.
    static func cards(_ count: Int) -> String {
        let formatted = count.formatted(.number.locale(.init(identifier: "fr_FR")))
        return count > 1 ? "\(formatted) cartes" : "\(formatted) carte"
    }

    /// "3 ouvertes" — the value on the "Filtres de rareté" row, as the mockup summarises it.
    static func openRarities(_ count: Int) -> String {
        switch count {
        case 0: "aucune ouverte"
        case 1: "1 ouverte"
        default: "\(count) ouvertes"
        }
    }

    /// The right-hand summary of a rarity row: what it actually offers today.
    static func proposed(_ count: Int, isOpen: Bool) -> String {
        guard isOpen else { return "aucun proposé" }
        let formatted = count.formatted(.number.locale(.init(identifier: "fr_FR")))
        return count > 1 ? "\(formatted) proposés" : "\(formatted) proposé"
    }
}
