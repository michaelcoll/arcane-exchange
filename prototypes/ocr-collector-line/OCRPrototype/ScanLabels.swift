import Foundation

/// The texts the screen shows (pure: kept out of the View so they can be tested).
enum ScanLabels {
    static func latency(_ latency: TimeInterval?) -> String {
        latency.map { String(format: "Lu en %.2f s", $0) } ?? "Pas encore de lecture"
    }

    static func mode(_ mode: ScanMode) -> String {
        switch mode {
        case .fixedStrip: "Bande fixe"
        case .cardContour: "Contour de la carte"
        }
    }

    /// Nil in the fixed strip mode, which does not look for the card.
    static func contour(_ contour: CardContour?, in mode: ScanMode) -> String? {
        guard mode == .cardContour else { return nil }
        return contour == nil ? "Aucune carte détectée" : "Carte détectée"
    }

    static func reading(_ reading: CollectorReading?) -> String {
        guard let reading else { return "-" }
        return "\(reading.set.uppercased()) n° \(reading.number) \(reading.language)"
    }

    static func lookup(_ state: LookupState?) -> String {
        switch state {
        case nil: return "Scryfall : aucune lecture à chercher"
        case .loading: return "Scryfall : recherche en cours..."
        case .notFound: return "Scryfall : introuvable (ni en anglais)"
        case let .failed(message): return "Scryfall : \(message)"
        case let .found(match): return found(match)
        }
    }

    private static func found(_ match: LookupMatch) -> String {
        let card = match.card
        var rows = [
            card.displayName,
            "\(card.set.uppercased()) n° \(card.number) \(card.language) - \(card.layout)",
        ]
        if !card.faceNames.isEmpty { rows.append("Faces : " + card.faceNames.joined(separator: " / ")) }
        if match.isEnglishPrinting { rows.append("Pas imprimé dans la langue lue : version anglaise affichée") }
        return rows.joined(separator: "\n")
    }
}
