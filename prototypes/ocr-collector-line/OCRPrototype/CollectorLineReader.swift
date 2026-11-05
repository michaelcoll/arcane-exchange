import Foundation

/// What the collector line of a card says. `set` is the printed set code, lowercased: for a token
/// the Scryfall set is "t" + `set` (`isToken`), and `number` has no leading zero.
struct CollectorReading: Equatable, Sendable {
    let set: String
    let number: String
    /// Scryfall language code.
    let language: String
    let isToken: Bool
}

/// Reads the collector line out of the text lines Vision recognised in one frame.
///
/// Two printed formats are understood (rarity letter `T` = token):
/// - 2014-2022: `0123/0280 R` then `DMU • FR  Artist`
/// - since 2023: `R 0123` then `MOM • EN`
///
/// A reading needs the set, the number and a known language together, otherwise nil.
enum CollectorLineReader {
    // "•" is often recognised as "*", "·" or "-". Digits may come out as O, I or l.
    private static let digit = "[0-9OIl]"
    private static let oldNumber = try! Regex("\\b(\(digit){3,4})\\s*/\\s*\(digit){3,4}\\s*([A-Z])?(?![a-z])")
    private static let newNumber = try! Regex("^([CURMLST])\\s+(\(digit){3,4})$")
    private static let setAndLanguage = try! Regex("\\b([A-Z0-9]{3,5})\\s*[•*·∙.\\-]\\s*([A-Z]{2})(?![A-Za-z])")

    /// Printed language code -> Scryfall language code.
    private static let languages = [
        "EN": "en", "FR": "fr", "DE": "de", "IT": "it", "SP": "es", "PT": "pt",
        "JP": "ja", "KR": "ko", "RU": "ru", "CS": "zhs", "CT": "zht", "PH": "ph",
    ]

    static func read(lines: [String]) -> CollectorReading? {
        guard let (set, language) = lines.lazy.compactMap(setAndLanguage(in:)).first,
              let (number, isToken) = lines.lazy.compactMap(number(in:)).first
        else { return nil }
        return CollectorReading(set: set, number: number, language: language, isToken: isToken)
    }

    private static func setAndLanguage(in line: String) -> (String, String)? {
        for match in line.matches(of: setAndLanguage) {
            guard let set = match.output[1].substring, let printed = match.output[2].substring,
                  let language = languages[String(printed)] else { continue }
            return (set.lowercased(), language)
        }
        return nil
    }

    private static func number(in line: String) -> (String, Bool)? {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        if let match = trimmed.firstMatch(of: newNumber),
           let rarity = match.output[1].substring, let digits = match.output[2].substring,
           let number = normalised(digits) {
            return (number, rarity == "T")
        }
        if let match = trimmed.firstMatch(of: oldNumber), let digits = match.output[1].substring,
           let number = normalised(digits) {
            return (number, match.output[2].substring == "T")
        }
        return nil
    }

    /// Fixes the O/I/l confusions, then drops the leading zeros. Nil when the number is 0.
    private static func normalised(_ digits: Substring) -> String? {
        let fixed = digits.map { "OIl".contains($0) ? ($0 == "O" ? "0" : "1") : String($0) }.joined()
        let number = String(fixed.drop { $0 == "0" })
        return number.isEmpty ? nil : number
    }
}
