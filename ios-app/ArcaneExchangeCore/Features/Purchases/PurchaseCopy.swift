import APIClient
import Foundation

package typealias Purchase = Components.Schemas.PurchaseResponse
package typealias SealedProductType = Components.Schemas.SealedProductTypeParam

/// The words of a purchase row, kept off the `View`: it is `@MainActor`, and tests call this.
package enum PurchaseCopy {
    package static func label(for type: SealedProductType) -> String {
        switch type {
        case .BOOSTER: "Booster"
        case .DISPLAY: "Display"
        case .BUNDLE: "Bundle"
        case .PRERELEASE_PACK: "Pack d'avant-première"
        case .PRECONSTRUCTED_DECK: "Deck préconstruit"
        }
    }

    /// "Type · Set", what a sealed product without a name is called; "Set · 4 × FR · Foil" for
    /// the copies of a single card.
    package static func kind(of purchase: Purchase) -> String {
        if let card = purchase.card {
            let parts: [String?] = [
                card.set.name,
                "\(purchase.quantity ?? 0) × \(card.language_code)",
                purchase.foil == true ? "Foil" : nil
            ]
            return parts.compactMap(\.self).joined(separator: " · ")
        }
        return [purchase.product_type.map(label(for:)), purchase.set?.name]
            .compactMap(\.self)
            .joined(separator: " · ")
    }

    /// The set of a purchase: the sealed product's, or the single card's.
    package static func set(of purchase: Purchase) -> SetInfo? {
        purchase.set ?? purchase.card?.set
    }

    /// The card of a single card purchase, else the name the player gave the sealed product,
    /// else "Type · Set".
    package static func title(of purchase: Purchase) -> String {
        purchase.card?.name ?? purchase.name ?? kind(of: purchase)
    }

    /// The date, preceded by what describes the purchase when the title does not: "Type · Set"
    /// under the player's own name, the copies under the name of a card.
    package static func subtitle(of purchase: Purchase) -> String {
        let date = PurchaseDate.display(purchase.purchased_on)
        return purchase.name == nil && purchase.card == nil ? date : "\(kind(of: purchase)) · \(date)"
    }

    /// How a sealed product reads in a list to pick from: its name, else "Type · Set", then its
    /// date.
    package static func choice(of purchase: Purchase) -> String {
        "\(title(of: purchase)) · \(PurchaseDate.display(purchase.purchased_on))"
    }

    /// The profitability, the value of the content minus the price, signed: "+8,50 € (+12 %)".
    package static func profitability(cents: Int, percent: Double) -> String {
        let rounded = Int(percent.rounded())
        return "\(sign(of: cents))\(Price.euros(cents: abs(cents))) (\(sign(of: rounded))\(abs(rounded)) %)"
    }

    /// "2 cartes sans prix de tendance, exclues de la valeur"; nil when none is left out.
    package static func withoutTrend(_ count: Int) -> String? {
        switch count {
        case ...0: nil
        case 1: "1 carte sans prix de tendance, exclue de la valeur"
        default: "\(count) cartes sans prix de tendance, exclues de la valeur"
        }
    }

    /// A real minus sign, like the web client; a plus only on a gain, nothing on zero.
    private static func sign(of value: Int) -> String {
        value < 0 ? "−" : (value > 0 ? "+" : "")
    }
}

/// The day of a purchase: `YYYY-MM-DD` on the wire, in the device's calendar (the app is only
/// used in France, so no time zone gymnastics).
package enum PurchaseDate {
    private static func formatter() -> DateFormatter {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = .current
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }

    package static func string(from date: Date) -> String {
        formatter().string(from: date)
    }

    package static func date(from string: String) -> Date? {
        formatter().date(from: string)
    }

    /// "13 nov. 2026"; the raw value when it is not a date.
    package static func display(_ string: String) -> String {
        guard let date = date(from: string) else { return string }
        return date.formatted(
            Date.FormatStyle(date: .abbreviated, time: .omitted, locale: Locale(identifier: "fr_FR"))
        )
    }
}

/// An amount in euros as the player types it, read as cents.
package enum EuroInput {
    /// "49,99", "49.99" and "50" read as 4999 and 5000 cents. Nil when it is not an amount, is not
    /// strictly positive, or has more than two decimals.
    package static func cents(from text: String) -> Int? {
        let normalized = text.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: ",", with: ".")
        guard normalized.wholeMatch(of: /\d+(\.\d+)?/) != nil,
              let euros = Decimal(string: normalized, locale: Locale(identifier: "en_US_POSIX"))
        else {
            return nil
        }
        let cents = euros * 100
        guard cents > 0, cents == cents.rounded(), cents <= Decimal(Int32.max) else { return nil }
        return NSDecimalNumber(decimal: cents).intValue
    }

    /// 4999 as "49,99", 5000 as "50": what the field shows when a purchase is edited.
    package static func text(cents: Int) -> String {
        (Decimal(cents) / 100).formatted(
            .number.precision(.fractionLength(0 ... 2)).grouping(.never).locale(Locale(identifier: "fr_FR"))
        )
    }
}

private extension Decimal {
    func rounded() -> Decimal {
        var value = self
        var result = Decimal()
        NSDecimalRound(&result, &value, 0, .plain)
        return result
    }
}
