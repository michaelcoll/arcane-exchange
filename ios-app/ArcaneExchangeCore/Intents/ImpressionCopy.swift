/// How an impression is named in the lists the system shows, kept off the entity so tests read it.
package enum ImpressionCopy {
    /// "Sol Ring · FDN · 001": the English name when it differs from the title, then the set
    /// and the number, which tell two impressions of one card apart.
    package static func subtitle(of card: SearchCard) -> String {
        let english = card.displayName == card.name ? nil : card.name
        return [english, ImpressionKey(card).label].compactMap(\.self).joined(separator: " · ")
    }
}
