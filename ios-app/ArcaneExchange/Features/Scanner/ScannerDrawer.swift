import ArcaneExchangeCore
import SwiftUI

/// The drawer at the bottom of the scanner: how many cards the scan batch holds and the last card
/// recognised. Open, it lists the batch, the most recent line first, with a stepper per line.
struct ScannerDrawer: View {
    let scanner: ScannerViewModel
    @Binding var isOpen: Bool
    let onClear: () -> Void
    @Environment(\.api) private var api
    @State private var correction: ScanCorrectionTarget?

    var body: some View {
        VStack(spacing: 0) {
            Button {
                withAnimation(.snappy) { isOpen.toggle() }
            } label: {
                header
            }
            .buttonStyle(.plain)
            .accessibilityValue(isOpen ? "Liste ouverte" : "Liste fermée")

            if isOpen {
                list
            } else {
                ScannerResultCard(card: scanner.lastRecognised)
                    .padding([.horizontal, .bottom], 12)
            }
        }
        .background(.regularMaterial, in: UnevenRoundedRectangle(topLeadingRadius: 24, topTrailingRadius: 24))
        .sheet(item: $correction) { target in
            ScanCorrectionSheet(target: target, scanner: scanner, api: api)
        }
    }

    private var header: some View {
        VStack(spacing: 10) {
            Capsule()
                .fill(.secondary.opacity(0.5))
                .frame(width: 36, height: 5)
            HStack {
                Text(summary)
                    .font(.headline)
                Spacer()
                Label(isOpen ? "Reprendre" : "Voir la liste", systemImage: isOpen ? "chevron.down" : "chevron.up")
                    .labelStyle(.titleAndIcon)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 8)
        .padding(.bottom, 12)
        .contentShape(Rectangle())
    }

    private var summary: String {
        let batch = scanner.batch
        let cards = batch.cardCount
        let unique = batch.lines.count
        return "\(cards) carte\(cards > 1 ? "s" : "") · \(unique) unique\(unique > 1 ? "s" : "")"
    }

    private var list: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Lot de scan")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Ajouter", systemImage: "plus") { correction = .add }
                    .font(.subheadline)
                    .padding(.trailing, 8)
                if scanner.batch.cardCount > 0 {
                    Button("Vider", role: .destructive, action: onClear)
                        .font(.subheadline)
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 8)

            if scanner.batch.lines.isEmpty {
                VStack(spacing: 8) {
                    Image(systemName: "viewfinder").font(.title)
                    Text("Aucune carte scannée pour l'instant.").font(.subheadline)
                }
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 32)
            } else {
                List(scanner.batch.lines) { line in
                    ScanBatchRow(line: line, quantity: Binding(
                        get: { line.quantity },
                        set: { scanner.setQuantity($0, ofLine: line.id) }
                    )) {
                        ScanLineMenu(line: line, scanner: scanner) { correction = .replace(lineID: line.id) }
                    }
                    .listRowBackground(Color.clear)
                }
                .listStyle(.plain)
                .frame(maxHeight: 420)
            }
        }
    }
}

/// A line of the scan batch: its card, its finish, and the stepper of its quantity.
private struct ScanBatchRow<Corrections: View>: View {
    let line: ScanBatchLine
    @Binding var quantity: Int
    @ViewBuilder let corrections: Corrections

    var body: some View {
        HStack(spacing: 12) {
            SetSymbol(setCode: line.set, size: 22)
                .foregroundStyle(RarityColor.tint(forRarityCode: line.rarity))
            VStack(alignment: .leading, spacing: 2) {
                Text(line.displayName)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(2)
                HStack(spacing: 6) {
                    Text("\(line.set) · #\(line.number) · \(line.language.rawValue)")
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                    if line.foil {
                        Text("Foil")
                            .font(.caption2.weight(.bold))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Palette.secondary.opacity(0.2), in: Capsule())
                            .foregroundStyle(Palette.secondaryInk)
                    }
                }
            }
            Spacer(minLength: 0)
            corrections
            Stepper(value: $quantity, in: 0 ... ScanBatch.maxQuantity) {
                Text("\(quantity)")
                    .font(.body.monospacedDigit())
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .frame(width: 150)
            .accessibilityLabel("Quantité de \(line.displayName)")
        }
    }
}
