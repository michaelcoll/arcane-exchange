import SwiftUI

/// Sub-drawer "Filtres de rareté" (the mockup's `sheet === 'rar'`): per rarity, whether it is
/// open to trade and how many copies are always kept.
struct TradeSettingsRaritiesSheet: View {
    let model: TradeSettingsViewModel

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    if model.rarities.isEmpty {
                        Text("Coche au moins un classeur pour voir les raretés que tu peux proposer.")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                    ForEach(model.rarities, id: \.rarity) { row in
                        rarityRow(row)
                    }
                    if !model.rarities.isEmpty {
                        TradeRatioBand(ratio: model.ratio)
                    }
                } footer: {
                    Text(
                        "Pour chaque rareté : est-elle ouverte à l'échange, et combien "
                            + "d'exemplaires tu gardes toujours pour toi."
                    )
                }
            }
            .navigationTitle("Filtres de rareté")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    CloseButton { dismiss() }
                }
            }
            .writeErrorAlert(model)
        }
        .presentationDetents([.large])
    }

    @ViewBuilder private func rarityRow(_ row: RarityFilter) -> some View {
        let isBusy = model.busyRarity == row.rarity
        let rarity = RarityCode(rawValue: row.rarity)

        VStack(alignment: .leading, spacing: 10) {
            Toggle(isOn: openBinding(row)) {
                // Same leading symbol as the collection's `CollectionRaritiesSheet`.
                HStack(spacing: 12) {
                    if let rarity {
                        RaritySymbol(rarity: rarity)
                    }
                    VStack(alignment: .leading, spacing: 2) {
                        Text(rarity?.label ?? row.rarity)
                        HStack(spacing: 6) {
                            Text(TradeSettingsCopy.copies(Int(row.copies)))
                            Text("·")
                            Text(TradeSettingsCopy.proposed(Int(row.proposed), isOpen: row.is_open))
                                .foregroundStyle(row.is_open ? Palette.primary : Color.secondary)
                        }
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    }
                }
            }

            if row.is_open {
                keptCopiesStepper(row)
            }
        }
        .padding(.vertical, 2)
        .disabled(isBusy)
        .opacity(isBusy ? 0.5 : 1)
    }

    private func openBinding(_ row: RarityFilter) -> Binding<Bool> {
        Binding(
            get: { row.is_open },
            set: { isOpen in
                // The kept count rides through untouched, like the web client's `toggleRarity`:
                // closing a rarity must not forget what the user had set aside on it.
                Task { await model.setRarity(row.rarity, isOpen: isOpen, keptCopies: row.kept_copies) }
            }
        )
    }

    /// How many copies of that rarity never leave, whatever the trade — the mockup's
    /// `IStepper`, capped at what the backend accepts.
    private func keptCopiesStepper(_ row: RarityFilter) -> some View {
        Stepper(
            value: Binding(
                get: { row.kept_copies },
                set: { kept in
                    Task { await model.setRarity(row.rarity, isOpen: true, keptCopies: kept) }
                }
            ),
            in: 0 ... TradeSettings.maxKeptCopies
        ) {
            HStack {
                Text("Exemplaires gardés")
                    .font(.subheadline)
                Spacer()
                Text("\(row.kept_copies)")
                    .font(.subheadline)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
            }
        }
    }
}
