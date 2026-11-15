import Foundation

/// The Possession axis of a search: which impressions count by what the player owns.
package enum PossessionAxis: CaseIterable, Hashable {
    case any
    case owned
    case notOwned

    package var label: String {
        switch self {
        case .any: "Toutes"
        case .owned: "Possédées"
        case .notOwned: "Manquantes"
        }
    }

    /// The `owned` query parameter: absent for every impression.
    var ownedQuery: Bool? {
        switch self {
        case .any: nil
        case .owned: true
        case .notOwned: false
        }
    }
}

/// The Offre axis of a search by name: the impressions some player offers, or the whole
/// Catalogue. A player's collection has no choice: the player is its Offre axis.
package enum OfferAxis: CaseIterable, Hashable {
    case offered
    case all

    package var label: String {
        switch self {
        case .offered: "Proposées"
        case .all: "Tout le catalogue"
        }
    }
}

/// The two independent axes of a search, combined: kept apart from `CollectionFilters`, which
/// the Collection tab shares and keeps on the device.
package struct SearchAxes: Hashable {
    package var possession: PossessionAxis
    package var offer: OfferAxis

    package init(possession: PossessionAxis = .any, offer: OfferAxis = .offered) {
        self.possession = possession
        self.offer = offer
    }
}
