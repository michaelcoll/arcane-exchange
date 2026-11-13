import Foundation

/// What the printed line of a card says (the glossary's ligne d'impression). `set` is the printed
/// set code, uppercased like the scan index's, `number` has no leading zero.
package struct PrintedLineReading: Hashable, Sendable {
    package let set: String
    package let number: String
    package let language: CardLanguage
    /// The rarity letter printed (`C`, `U`, `R`, `M`, `L`, `S`, `T`); nil when the line did not give it.
    package let rarity: String?

    package init(set: String, number: String, language: CardLanguage, rarity: String?) {
        self.set = set
        self.number = number
        self.language = language
        self.rarity = rarity
    }
}

/// Reads the printed line out of the text lines Vision recognised in one frame.
///
/// Two printed formats are understood:
/// - 2014-2022: `0123/0280 R` then `DMU • FR  Artist`
/// - since 2023: `R 0123` then `MOM • EN`
///
/// A reading needs the set, the number (purely numeric) and a language the domain knows together,
/// otherwise nil. The rarity letter is kept, not judged: a token (`T`) is dropped later by
/// `CardRecogniser`, which compares it with the scan index.
package enum PrintedLineReader {
    /// The regexes are immutable once built and read from the camera queue: `nonisolated(unsafe)` only
    /// tells Swift 6 what is true.
    /// "•" is often recognised as "*", "·" or "-". Digits may come out as O, I or l.
    private static let digit = "[0-9OIl]"
    // swiftlint:disable force_try
    private nonisolated(unsafe) static let oldNumber = try! Regex(
        "\\b(\(digit){3,4})\\s*/\\s*\(digit){3,4}\\s*([A-Z])?(?![a-z])"
    )
    /// Not anchored: Vision may merge "R 0123" with the set line or the artist. The lookahead asks for
    /// one real digit so that a word like "S Ill" is not a number.
    private nonisolated(unsafe) static let newNumber = try! Regex(
        "\\b([CURMLST])\\s+(?=[OIl]*[0-9])(\(digit){3,4})\\b"
    )
    private nonisolated(unsafe) static let setAndLanguage = try! Regex(
        "\\b([A-Z0-9]{3,5})\\s*[•*·∙.\\-]\\s*([A-Z]{2})(?![A-Za-z])"
    )
    // swiftlint:enable force_try

    /// Printed language code -> language of a copy. A language the domain does not know is no reading.
    private static let languages: [String: CardLanguage] = [
        "EN": .english, "FR": .french, "DE": .german, "IT": .italian, "SP": .spanish, "JP": .japanese
    ]

    package static func read(lines: [String]) -> PrintedLineReading? {
        guard let (set, language) = lines.lazy.compactMap(setAndLanguage(in:)).first,
              let (number, rarity) = lines.lazy.compactMap(number(in:)).first
        else { return nil }
        return PrintedLineReading(set: set, number: number, language: language, rarity: rarity)
    }

    private static func setAndLanguage(in line: String) -> (String, CardLanguage)? {
        for match in line.matches(of: setAndLanguage) {
            guard let set = match.output[1].substring, let printed = match.output[2].substring,
                  let language = languages[String(printed)] else { continue }
            return (set.uppercased(), language)
        }
        return nil
    }

    private static func number(in line: String) -> (String, String?)? {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        return oldFormatNumber(in: trimmed) ?? newFormatNumber(in: trimmed)
    }

    /// `0123/0280 R`: the rarity comes last, and may be cut off.
    private static func oldFormatNumber(in line: String) -> (String, String?)? {
        guard let match = line.firstMatch(of: oldNumber), let digits = match.output[1].substring else { return nil }
        return normalised(digits).map { ($0, match.output[2].substring.map(String.init)) }
    }

    /// `R 0123`: the rarity comes first.
    private static func newFormatNumber(in line: String) -> (String, String?)? {
        guard let match = line.firstMatch(of: newNumber),
              let rarity = match.output[1].substring, let digits = match.output[2].substring
        else { return nil }
        return normalised(digits).map { ($0, String(rarity)) }
    }

    /// Fixes the O/I/l confusions, then drops the leading zeros. Nil when the number is 0.
    private static func normalised(_ digits: Substring) -> String? {
        let fixed = digits.map { "OIl".contains($0) ? ($0 == "O" ? "0" : "1") : String($0) }.joined()
        let number = String(fixed.drop { $0 == "0" })
        return number.isEmpty ? nil : number
    }
}
