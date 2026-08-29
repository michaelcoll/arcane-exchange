import Foundation

enum Price {
    /// Formats a price stored in cents the way the web client does (`utils/format-price.ts`):
    /// fr-FR euros, decimals only when the amount actually has any.
    ///
    /// The locale is pinned rather than taken from the device: the whole UI is French and the
    /// backend only ever deals in euros, so a device set to another region should not end up
    /// reading "€31.00" next to "31 €" elsewhere in the product.
    static func euros(cents: Int) -> String {
        (Decimal(cents) / 100).formatted(
            .currency(code: "EUR")
                .precision(.fractionLength(0 ... 2))
                .locale(Locale(identifier: "fr_FR"))
        )
    }
}

/// Compares what a card is worth today against what it was bought for.
///
/// Mirrors `Card/Cell.vue`'s `dealInfo`: a swing under 3 % is noise, not a deal.
struct CardDeal: Equatable {
    enum Kind: Equatable {
        case good
        case bad
        case par
    }

    let percent: Int
    let kind: Kind

    init?(purchaseCents: Int?, trendCents: Int?) {
        guard let purchase = purchaseCents, purchase > 0, let trend = trendCents else { return nil }
        let percent = Int(((Double(trend) - Double(purchase)) / Double(purchase) * 100).rounded())
        self.percent = percent
        kind = percent >= 3 ? .good : (percent <= -3 ? .bad : .par)
    }

    /// Signed percentage, using a real minus sign like the web client.
    var label: String {
        percent <= 0 ? "−\(abs(percent))%" : "+\(percent)%"
    }
}
