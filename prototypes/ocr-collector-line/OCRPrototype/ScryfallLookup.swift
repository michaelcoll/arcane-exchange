import Foundation

/// One request of a lookup: `isEnglishPrinting` when the language read is not the one asked for.
struct LookupStep: Equatable, Sendable {
    let url: URL
    let isEnglishPrinting: Bool
}

/// The Scryfall Card found, trimmed to what the screen shows.
struct ScryfallCard: Equatable, Sendable {
    let name: String
    let printedName: String?
    let set: String
    let number: String
    let language: String
    let layout: String
    /// Face names of a double-faced Card (empty for a single-faced one).
    let faceNames: [String]

    var displayName: String { printedName ?? name }
}

/// Pure part of the Scryfall lookup by set + number (no network).
enum ScryfallLookup {
    private static let base = "https://api.scryfall.com/cards"

    /// The requests, tried in order until one finds the Card: the language read, then English
    /// (some printings are missing in the language read).
    static func plan(for reading: CollectorReading) -> [LookupStep] {
        var steps = [LookupStep(url: url(reading.set, reading.number, reading.language), isEnglishPrinting: false)]
        if reading.language != "en" {
            steps.append(LookupStep(url: url(reading.set, reading.number, nil), isEnglishPrinting: true))
        }
        return steps
    }

    private static func url(_ set: String, _ number: String, _ language: String?) -> URL {
        let number = number.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? number
        return URL(string: "\(base)/\(set)/\(number)" + (language.map { "/\($0)" } ?? ""))!
    }

    /// The Card of a Scryfall response body, nil for an error body (404...) or anything else.
    static func decode(_ data: Data) -> ScryfallCard? {
        guard let dto = try? JSONDecoder().decode(CardDTO.self, from: data), dto.object == "card" else { return nil }
        return ScryfallCard(
            name: dto.name, printedName: dto.printed_name, set: dto.set, number: dto.collector_number,
            language: dto.lang, layout: dto.layout, faceNames: dto.card_faces?.map(\.name) ?? [])
    }

    private struct CardDTO: Decodable {
        let object: String
        let name: String
        let printed_name: String?
        let set: String
        let collector_number: String
        let lang: String
        let layout: String
        let card_faces: [FaceDTO]?
    }

    private struct FaceDTO: Decodable {
        let name: String
    }
}
