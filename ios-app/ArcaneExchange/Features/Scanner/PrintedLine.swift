import ArcaneExchangeCore
import SwiftUI

/// A card's references in the order of its printed line, in the monospaced font of the given text
/// style: "123 MOM · FR", a small star in place of the dot for a foil, as on the card.
struct PrintedLine: View {
    let number: String
    let set: String
    let foil: Bool
    let language: CardLanguage
    let style: Font.TextStyle
    /// The size of the text style, following Dynamic Type.
    @ScaledMetric private var textSize: CGFloat

    init(number: String, set: String, foil: Bool, language: CardLanguage, style: Font.TextStyle) {
        self.number = number
        self.set = set
        self.foil = foil
        self.language = language
        self.style = style
        _textSize = ScaledMetric(wrappedValue: TextStyleSize.standard(of: style), relativeTo: style)
    }

    var body: some View {
        Text("\(number) \(set) \(foil ? star : Text("·")) \(language.rawValue)")
            .font(.system(style, design: .monospaced))
    }

    /// Half the text size, lifted so its centre stays on the dot's.
    private var star: Text {
        let size = textSize / 2
        return Text(Image(systemName: "star.fill"))
            .font(.system(size: size))
            .baselineOffset(size * 0.35)
    }
}

#Preview {
    VStack(alignment: .leading) {
        PrintedLine(number: "123", set: "MOM", foil: false, language: .french, style: .subheadline)
        PrintedLine(number: "123", set: "MOM", foil: true, language: .french, style: .subheadline)
        PrintedLine(number: "123", set: "MOM", foil: false, language: .french, style: .caption)
        PrintedLine(number: "123", set: "MOM", foil: true, language: .french, style: .caption)
    }
    .foregroundStyle(.secondary)
}
