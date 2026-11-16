/// What Siri answers to "do I own this card": the owned quantity of the impression, all
/// languages and finishes together. Kept off the intent so tests read it.
package enum OwnedCopy {
    package static func text(of card: SearchCard) -> String {
        switch card.owned_quantity {
        case ...0: "Tu n'as pas \(card.displayName)."
        case 1: "Tu as 1 exemplaire de \(card.displayName)."
        default: "Tu as \(card.owned_quantity) exemplaires de \(card.displayName)."
        }
    }
}
