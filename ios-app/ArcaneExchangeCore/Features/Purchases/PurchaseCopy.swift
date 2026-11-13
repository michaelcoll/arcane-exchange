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

    /// "Type · Set", what a sealed product without a name is called.
    package static func kind(of purchase: Purchase) -> String {
        [purchase.product_type.map(label(for:)), purchase.set?.name]
            .compactMap(\.self)
            .joined(separator: " · ")
    }

    /// The name the player gave the sealed product, else "Type · Set".
    package static func title(of purchase: Purchase) -> String {
        purchase.name ?? kind(of: purchase)
    }

    /// The date, preceded by "Type · Set" when the title is the player's own name.
    package static func subtitle(of purchase: Purchase) -> String {
        let date = PurchaseDate.display(purchase.purchased_on)
        return purchase.name == nil ? date : "\(kind(of: purchase)) · \(date)"
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
