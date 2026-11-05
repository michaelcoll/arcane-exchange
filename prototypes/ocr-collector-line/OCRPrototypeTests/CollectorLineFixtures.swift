@testable import OCRPrototype

/// SYNTHETIC FIXTURES. No physical card was available: these lines are written from the known
/// printed formats of the collector line, with typical OCR confusions, NOT captured by Vision.
/// Replace or extend them with real captures (`Copy reading` in the app) in #658.
struct CollectorLineFixture: Sendable {
    let name: String
    let lines: [String]
    let expected: CollectorReading?
}

enum CollectorLineFixtures {
    static let all: [CollectorLineFixture] = [
        // Format 2014-2022: "0123/0280 R" then "DMU • FR  Artist".
        .init(name: "2014-2022, French",
              lines: ["Éclair", "Éphémère", "0123/0280 R", "DMU • FR  Jean Dupont", "TM & © 2022 Wizards of the Coast"],
              expected: .init(set: "dmu", number: "123", language: "fr", isToken: false)),
        .init(name: "2014-2022, English",
              lines: ["Lightning Bolt", "Instant", "0150/0274 M", "M21 • EN  John Smith"],
              expected: .init(set: "m21", number: "150", language: "en", isToken: false)),
        .init(name: "2014-2022, separator read as asterisk",
              lines: ["0123/0280 R", "DMU * FR  Jean Dupont"],
              expected: .init(set: "dmu", number: "123", language: "fr", isToken: false)),
        .init(name: "2014-2022, separator read as middle dot",
              lines: ["0123/0280 R", "DMU · FR Jean Dupont"],
              expected: .init(set: "dmu", number: "123", language: "fr", isToken: false)),
        .init(name: "2014-2022, separator read as hyphen",
              lines: ["0123/0280 U", "DMU - EN Jean Dupont"],
              expected: .init(set: "dmu", number: "123", language: "en", isToken: false)),
        .init(name: "2014-2022, O for 0 and I for 1 in the number",
              lines: ["O1I3/O28O R", "DMU • FR  Jean Dupont"],
              expected: .init(set: "dmu", number: "113", language: "fr", isToken: false)),
        .init(name: "2014-2022, both on one line",
              lines: ["0123/0280 R DMU • FR  Jean Dupont"],
              expected: .init(set: "dmu", number: "123", language: "fr", isToken: false)),
        .init(name: "2014-2022, token",
              lines: ["0005/0010 T", "DMU • EN  Jean Dupont"],
              expected: .init(set: "dmu", number: "5", language: "en", isToken: true)),
        // Format since 2023: "R 0123" then "MOM • EN".
        .init(name: "since 2023, English",
              lines: ["Elesh Norn", "R 0123", "MOM • EN", "Jean Dupont"],
              expected: .init(set: "mom", number: "123", language: "en", isToken: false)),
        .init(name: "since 2023, French",
              lines: ["R 0123", "MOM • FR", "Jean Dupont"],
              expected: .init(set: "mom", number: "123", language: "fr", isToken: false)),
        .init(name: "since 2023, token",
              lines: ["T 0016", "MOM • EN"],
              expected: .init(set: "mom", number: "16", language: "en", isToken: true)),
        .init(name: "since 2023, O read for 0",
              lines: ["U O123", "MKM * EN"],
              expected: .init(set: "mkm", number: "123", language: "en", isToken: false)),
        .init(name: "Japanese printed code",
              lines: ["R 0123", "MOM • JP"],
              expected: .init(set: "mom", number: "123", language: "ja", isToken: false)),
        .init(name: "Spanish printed code",
              lines: ["R 0123", "MOM • SP"],
              expected: .init(set: "mom", number: "123", language: "es", isToken: false)),
        // Not readable: no reading.
        .init(name: "no collector line",
              lines: ["Lightning Bolt", "Instant", "Lightning Bolt deals 3 damage to any target."],
              expected: nil),
        .init(name: "old-format card text (power/toughness, no collector line)",
              lines: ["Llanowar Elves", "Creature — Elf Druid", "T: Add G.", "1/1", "ILLUS. Anson Maddocks", "© 1993-1994 Wizards of the Coast"],
              expected: nil),
        .init(name: "set and language without a number",
              lines: ["DMU • FR  Jean Dupont"],
              expected: nil),
        .init(name: "number without set and language",
              lines: ["0123/0280 R"],
              expected: nil),
        .init(name: "unknown language code",
              lines: ["R 0123", "MOM • XX"],
              expected: nil),
        .init(name: "empty frame",
              lines: [],
              expected: nil),
    ]
}
