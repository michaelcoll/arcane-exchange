import Foundation

/// The texts the screen shows (pure: kept out of the View so they can be tested).
enum ScanLabels {
    static func latency(_ latency: TimeInterval?) -> String {
        latency.map { String(format: "Lu en %.2f s", $0) } ?? "Pas encore de lecture"
    }

    static func reading(_ reading: CollectorReading?) -> String {
        guard let reading else { return "-" }
        let rarity = reading.isToken ? "rareté T : token" : "rareté autre que T ou absente : carte ou token"
        return "\(reading.set.uppercased()) n° \(reading.number) \(reading.language) (\(rarity))"
    }

    static func lookup(_ state: LookupState?) -> String {
        switch state {
        case nil: return "Scryfall : aucune lecture à chercher"
        case .loading: return "Scryfall : recherche en cours..."
        case .notFound: return "Scryfall : introuvable (ni en anglais)"
        case let .failed(message): return "Scryfall : \(message)"
        case let .found(matches): return matches.map(match).joined(separator: "\n\n")
        }
    }

    private static func match(_ match: LookupMatch) -> String {
        let card = match.card
        var rows = [
            (match.isToken ? "TOKEN" : "CARTE") + " : " + card.displayName,
            "\(card.set.uppercased()) n° \(card.number) \(card.language) - \(card.layout)",
        ]
        if !card.faceNames.isEmpty { rows.append("Faces : " + card.faceNames.joined(separator: " / ")) }
        if match.isEnglishPrinting { rows.append("Pas imprimé dans la langue lue : version anglaise affichée") }
        return rows.joined(separator: "\n")
    }
}
