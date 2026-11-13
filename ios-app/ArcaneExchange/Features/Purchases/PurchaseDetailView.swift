import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The detail of a sealed product: what it contained, the value of that content at today's trend
/// prices, and its profitability against its price.
struct PurchaseDetailView: View {
    @State private var model: PurchaseDetailViewModel

    init(purchase: Purchase, api: any APIProtocol) {
        _model = State(initialValue: PurchaseDetailViewModel(purchase: purchase, api: api))
    }

    var body: some View {
        List {
            Section {
                LabeledContent("Prix", value: Price.euros(cents: Int(model.purchase.amount_cents)))
                if let value = model.valueLabel, let profitability = model.profitabilityLabel {
                    LabeledContent("Valeur du contenu", value: value)
                    LabeledContent("Rentabilité", value: profitability)
                }
            } header: {
                Text(PurchaseCopy.subtitle(of: model.purchase))
            } footer: {
                if let withoutTrend = model.withoutTrendLabel {
                    Text(withoutTrend)
                }
            }
            if let detail = model.detail {
                Section("Contenu") {
                    if detail.content.isEmpty {
                        Text("Aucune carte rattachée pour l'instant.")
                            .foregroundStyle(.secondary)
                    }
                    ForEach(detail.content, id: \.self) { line in
                        row(line)
                    }
                }
            }
        }
        .overlay {
            if let message = model.loadError {
                ContentUnavailableView {
                    Label("Achat indisponible", systemImage: "exclamationmark.triangle")
                } description: {
                    Text(message)
                } actions: {
                    Button("Réessayer") { Task { await model.load() } }
                }
            } else if !model.hasLoaded {
                ProgressView()
                    .controlSize(.large)
            }
        }
        .navigationTitle(PurchaseCopy.title(of: model.purchase))
        .navigationBarTitleDisplayMode(.inline)
        .task { await model.load() }
        .refreshable { await model.load() }
    }

    private func row(_ line: PurchaseContentLine) -> some View {
        HStack(spacing: 12) {
            CardImageView(url: CardArtwork.url(imagePath: line.image_url), foil: line.foil)
                .frame(width: 40, height: 56)
                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
            VStack(alignment: .leading, spacing: 2) {
                Text(line.name)
                    .lineLimit(1)
                Text("\(line.set_code) · \(line.collector_number) · \(line.language_code)\(line.foil ? " · Foil" : "")")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 2) {
                Text("×\(line.quantity)")
                    .font(.body.monospacedDigit())
                Text(line.trend_cents.map { Price.euros(cents: Int($0)) } ?? "—")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }
        }
    }
}

#Preview {
    NavigationStack {
        PurchaseDetailView(
            purchase: Purchase(
                amount_cents: 499,
                id: "p-1",
                kind: .sealed_product,
                name: nil,
                product_type: .BOOSTER,
                purchased_on: "2026-11-13",
                set: .init(code: "FDN", name: "Foundations")
            ),
            api: APIClientFactory.preview
        )
    }
}
