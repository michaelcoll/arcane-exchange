import Foundation

package enum Price {
    /// Formats a price stored in cents the way the web client does (`utils/format-price.ts`):
    /// fr-FR euros, decimals only when the amount actually has any.
    ///
    /// The locale is pinned rather than taken from the device: the whole UI is French and the
    /// backend only ever deals in euros, so a device set to another region should not end up
    /// reading "€31.00" next to "31 €" elsewhere in the product.
    package static func euros(cents: Int) -> String {
        (Decimal(cents) / 100).formatted(
            .currency(code: "EUR")
                .precision(.fractionLength(0 ... 2))
                .locale(Locale(identifier: "fr_FR"))
        )
    }

    /// Reads an amount typed in euros ("2", "2,5", "2.50") as cents; nil when empty or not an amount
    /// with at most two decimals. A zero is read as 0: refusing it is the backend's rule.
    package static func cents(fromEuros text: String) -> Int? {
        let trimmed = text.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: ",", with: ".")
        let parts = trimmed.split(separator: ".", omittingEmptySubsequences: false)
        guard (1 ... 2).contains(parts.count),
              parts.allSatisfy({ $0.allSatisfy(\.isASCII) && $0.allSatisfy(\.isNumber) }),
              !parts[0].isEmpty,
              parts.count == 1 || (1 ... 2).contains(parts[1].count)
        else { return nil }
        let euros = Int(parts[0]) ?? 0
        let decimals = parts.count == 2 ? (Int(parts[1]) ?? 0) * (parts[1].count == 1 ? 10 : 1) : 0
        return euros * 100 + decimals
    }
}

/// Compares what a card is worth today against what it was bought for.
///
/// Mirrors the web `utils/purchase-spread.ts`'s `computePurchaseSpread`: a swing under 3 % is noise, not a spread.
package struct PurchaseSpread: Equatable {
    package enum Kind: Equatable {
        case gain
        case loss
        case par
    }

    package let percent: Int
    package let kind: Kind

    package init?(purchaseCents: Int?, trendCents: Int?) {
        guard let purchase = purchaseCents, purchase > 0, let trend = trendCents else { return nil }
        let percent = Int(((Double(trend) - Double(purchase)) / Double(purchase) * 100).rounded())
        self.percent = percent
        kind = percent >= 3 ? .gain : (percent <= -3 ? .loss : .par)
    }

    /// Signed percentage, using a real minus sign like the web client.
    package var label: String {
        percent <= 0 ? "−\(abs(percent))%" : "+\(percent)%"
    }
}
