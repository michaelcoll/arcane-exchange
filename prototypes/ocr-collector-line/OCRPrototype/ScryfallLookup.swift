import Foundation

/// One request of a lookup: `isEnglishPrinting` when the language read is not the one asked for.
struct LookupStep: Equatable, Sendable {
    let url: URL
    let isEnglishPrinting: Bool
}

/// The requests for one candidate: the Card set (`mkm/12`) or the token set (`tmkm/12`).
struct LookupTarget: Equatable, Sendable {
    let isToken: Bool
    let steps: [LookupStep]
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

    /// Candidates to look up. The rarity letter `T` says token: only the token set ("t" + printed
    /// set). Otherwise (other letter or none) both the Card set and the token set, because the
    /// printed code is the same (`mkm/12` is a Card, `tmkm/12` a token).
    /// A Card set tries the language read, then English (some printings are missing). Scryfall has
    /// no non-English token: a token set goes straight to English.
    static func plan(for reading: CollectorReading) -> [LookupTarget] {
        var targets: [LookupTarget] = []
        if !reading.isToken {
            var steps = [LookupStep(url: url(reading.set, reading.number, reading.language), isEnglishPrinting: false)]
            if reading.language != "en" {
                steps.append(LookupStep(url: url(reading.set, reading.number, nil), isEnglishPrinting: true))
            }
            targets.append(LookupTarget(isToken: false, steps: steps))
        }
        let tokenStep = LookupStep(url: url("t" + reading.set, reading.number, nil), isEnglishPrinting: reading.language != "en")
        targets.append(LookupTarget(isToken: true, steps: [tokenStep]))
        return targets
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
