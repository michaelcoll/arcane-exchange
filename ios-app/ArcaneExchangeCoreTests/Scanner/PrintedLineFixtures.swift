@testable import ArcaneExchangeCore

/// SYNTHETIC FIXTURES, taken over from the prototype (`prototypes/ocr-collector-line`). No physical
/// card was available: these lines are written from the known printed formats of the printed line,
/// with typical OCR confusions, NOT captured by Vision. Texts captured on real cards come in their
/// own ticket.
struct PrintedLineFixture: Sendable {
    let name: String
    let lines: [String]
    let expected: PrintedLineReading?
}

private func reading(_ set: String, _ number: String, _ language: CardLanguage, _ rarity: String? = nil)
    -> PrintedLineReading {
    PrintedLineReading(set: set, number: number, language: language, rarity: rarity)
}

enum PrintedLineFixtures {
    static let all: [PrintedLineFixture] = [
        // Format 2014-2022: "0123/0280 R" then "DMU • FR  Artist".
        .init(name: "2014-2022, French",
              lines: ["Éclair", "Éphémère", "0123/0280 R", "DMU • FR  Jean Dupont", "TM & © 2022 Wizards of the Coast"],
              expected: reading("DMU", "123", .french, "R")),
        .init(name: "2014-2022, English",
              lines: ["Lightning Bolt", "Instant", "0150/0274 M", "M21 • EN  John Smith"],
              expected: reading("M21", "150", .english, "M")),
        .init(name: "2014-2022, separator read as asterisk",
              lines: ["0123/0280 R", "DMU * FR  Jean Dupont"],
              expected: reading("DMU", "123", .french, "R")),
        .init(name: "2014-2022, separator read as middle dot",
              lines: ["0123/0280 R", "DMU · FR Jean Dupont"],
              expected: reading("DMU", "123", .french, "R")),
        .init(name: "2014-2022, separator read as hyphen",
              lines: ["0123/0280 U", "DMU - EN Jean Dupont"],
              expected: reading("DMU", "123", .english, "U")),
        .init(name: "2014-2022, O for 0 and I for 1 in the number",
              lines: ["O1I3/O28O R", "DMU • FR  Jean Dupont"],
              expected: reading("DMU", "113", .french, "R")),
        .init(name: "2014-2022, both on one line",
              lines: ["0123/0280 R DMU • FR  Jean Dupont"],
              expected: reading("DMU", "123", .french, "R")),
        .init(name: "2014-2022, token: rarity T",
              lines: ["0005/0010 T", "DMU • EN  Jean Dupont"],
              expected: reading("DMU", "5", .english, "T")),
        // Format since 2023: "R 0123" then "MOM • EN".
        .init(name: "since 2023, English",
              lines: ["Elesh Norn", "R 0123", "MOM • EN", "Jean Dupont"],
              expected: reading("MOM", "123", .english, "R")),
        .init(name: "since 2023, French",
              lines: ["R 0123", "MOM • FR", "Jean Dupont"],
              expected: reading("MOM", "123", .french, "R")),
        .init(name: "since 2023, token: rarity T",
              lines: ["T 0016", "MOM • EN"],
              expected: reading("MOM", "16", .english, "T")),
        .init(name: "since 2023, O read for 0",
              lines: ["U O123", "MKM * EN"],
              expected: reading("MKM", "123", .english, "U")),
        .init(name: "since 2023, merged with the artist on one line",
              lines: ["Elesh Norn", "R 0123 MOM • EN  Jean Dupont"],
              expected: reading("MOM", "123", .english, "R")),
        .init(name: "since 2023, merged with text before it",
              lines: ["Jean Dupont R 0123", "MOM • FR"],
              expected: reading("MOM", "123", .french, "R")),
        .init(name: "ligne coupée, artist cut off after the language",
              lines: ["R 0123", "MOM • EN  Jean Du"],
              expected: reading("MOM", "123", .english, "R")),
        .init(name: "ligne coupée, number cut off",
              lines: ["R 01", "MOM • EN"],
              expected: nil),
        .init(name: "ligne coupée, language cut off",
              lines: ["R 0123", "MOM • E"],
              expected: nil),
        .init(name: "ligne coupée, old-format total cut off",
              lines: ["0123/02", "DMU • FR"],
              expected: nil),
        .init(name: "rarity letter inside a word is not a rarity",
              lines: ["ITEM 0123", "MOM • EN"],
              expected: nil),
        .init(name: "Japanese printed code",
              lines: ["R 0123", "MOM • JP"],
              expected: reading("MOM", "123", .japanese, "R")),
        .init(name: "Spanish printed code",
              lines: ["R 0123", "MOM • SP"],
              expected: reading("MOM", "123", .spanish, "R")),
        // Not readable: no reading.
        .init(name: "non numeric collector number",
              lines: ["R H013", "MOM • EN"],
              expected: nil),
        .init(name: "no printed line",
              lines: ["Lightning Bolt", "Instant", "Lightning Bolt deals 3 damage to any target."],
              expected: nil),
        .init(name: "old-format card text (power/toughness, no printed line)",
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
        .init(name: "language the domain does not know",
              lines: ["R 0123", "MOM • RU"],
              expected: nil),
        .init(name: "empty frame",
              lines: [],
              expected: nil),
    ]
}
