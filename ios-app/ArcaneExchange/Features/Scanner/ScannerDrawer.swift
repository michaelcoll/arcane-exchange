import ArcaneExchangeCore
import NukeUI
import SwiftUI

extension PresentationDetent {
    /// The scanner drawer folded: the batch total, the last card recognised, and the close and
    /// « Terminer » buttons.
    static let scannerCollapsed = PresentationDetent.height(ScannerDrawer.collapsedHeight)
}

/// The scanner drawer, a native sheet that never leaves the Scanner tab. Folded, it shows how many
/// cards the scan batch holds, the Foil switch and the last card recognised, the camera staying
/// usable above it.
/// Pulled up, it lists the batch, the most recent line first, with a stepper and the corrections
/// per line. At the bottom, always: the button that closes the scanner, and the one that sends the
/// batch to the collection.
struct ScannerDrawer: View {
    /// The height of the folded drawer, kept clear by what the scanner shows behind it.
    nonisolated static let collapsedHeight: CGFloat = 232

    let scanner: ScannerViewModel
    let validation: ScanValidationViewModel
    @Binding var detent: PresentationDetent
    /// Leaves the scanner.
    let onClose: () -> Void
    @Environment(\.api) private var api
    @State private var correction: ScanCorrectionTarget?
    @State private var isConfirmingClear = false
    @State private var isValidating = false

    private var isOpen: Bool {
        detent != .scannerCollapsed
    }

    var body: some View {
        VStack(spacing: 0) {
            header

            if isOpen {
                list
            } else {
                ScannerResultCard(card: scanner.lastRecognised, foil: scanner.lastRecognisedFoil)
                Spacer(minLength: 0)
            }
            actions
        }
        .presentationDetents([.scannerCollapsed, .medium, .large], selection: $detent)
        .presentationDragIndicator(.visible)
        // Folded, the camera and the toast stay usable above the drawer.
        .presentationBackgroundInteraction(.enabled(upThrough: .scannerCollapsed))
        .interactiveDismissDisabled()
        // Nothing is added while the list, the validation or the confirmation is open.
        .onChange(of: isOpen || isValidating || isConfirmingClear, initial: true) { _, paused in
            scanner.isPaused = paused
        }
        .sheet(item: $correction) { target in
            ScanCorrectionSheet(target: target, scanner: scanner, api: api)
        }
        .sheet(isPresented: $isValidating) {
            ScanValidationSheet(
                model: validation,
                cardCount: scanner.batch.cardCount,
                valueCents: scanner.batchValue?.cents,
                // Back to the camera.
                onValidated: { detent = .scannerCollapsed },
                // The refused lines are flagged in the list.
                onRefused: { detent = .large }
            )
        }
        .alert("Vider le lot ?", isPresented: $isConfirmingClear) {
            Button("Annuler", role: .cancel) {}
            Button("Vider", role: .destructive) { scanner.clear() }
        } message: {
            Text("Les cartes scannées seront retirées du lot. Ta collection n'est pas modifiée.")
        }
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(ScanBatchCopy.cards(scanner.batch.cardCount))
                    .font(.headline)
                if let value = scanner.batchValue {
                    Text(ScanBatchCopy.valueText(cents: value.cents, unpricedLines: value.unpricedLines))
                        .font(.subheadline.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
            }
            Spacer()
            // Folded too: the camera reads while the drawer is folded.
            FoilButton(isOn: Bindable(scanner).foil)
            if isOpen, scanner.batch.cardCount > 0 {
                Button("Vider le lot", systemImage: "trash", role: .destructive) { isConfirmingClear = true }
                    .labelStyle(.iconOnly)
                    .font(.body.weight(.medium))
                    .foregroundStyle(.red)
                    .modifier(NeutralCapsuleButtonStyle())
            }
            if isOpen {
                // A card the camera does not read, picked in the catalog.
                Button("Ajouter") { correction = .add }
                    .font(.body.weight(.medium))
                    // Neutral, like the toolbar buttons of the sheets: the accent colour stays for
                    // the validation.
                    .foregroundStyle(.primary)
                    .modifier(NeutralCapsuleButtonStyle())
                    .accessibilityLabel("Ajouter une carte")
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 20)
        .padding(.bottom, 12)
    }

    /// The button that opens the validation, and the close button on the right, under the thumb.
    private var actions: some View {
        HStack(spacing: 8) {
            Button {
                isValidating = true
            } label: {
                Text(isOpen ? ScanBatchCopy.addToCollection(cards: scanner.batch.cardCount) : "Terminer")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(scanner.batch.lines.isEmpty)
            Button("Fermer le scanner", systemImage: "xmark", action: onClose)
                .labelStyle(.iconOnly)
                .font(.body.weight(.medium))
                .foregroundStyle(.primary)
                // Set closer to the button than the capsule of the style, so it wins.
                .buttonBorderShape(.circle)
                .modifier(NeutralCapsuleButtonStyle())
        }
        .controlSize(.large)
        .padding(.horizontal, 12)
        .padding(.top, 10)
        .padding(.bottom, 12)
    }

    private var list: some View {
        VStack(spacing: 0) {
            if scanner.batch.lines.isEmpty {
                ContentUnavailableView(
                    "Aucune carte scannée",
                    systemImage: "viewfinder",
                    description: Text("Place une carte dans le cadre, ou ajoute-la depuis le catalogue.")
                )
            } else {
                List(scanner.batch.lines) { line in
                    ScanBatchRow(
                        line: line,
                        scanner: scanner,
                        isRejected: validation.rejectedLineIDs.contains(line.id),
                        cents: scanner.lineCents(of: line),
                        quantity: Binding(
                            get: { line.quantity },
                            set: { scanner.setQuantity($0, ofLine: line.id) }
                        )
                    ) {
                        correction = .replace(lineID: line.id)
                    }
                    .listRowBackground(Color.clear)
                }
                .listStyle(.plain)
                .scrollContentBackground(.hidden)
            }
        }
    }
}

/// The glass capsule of the toolbar buttons where the system has it, a grey capsule before.
private struct NeutralCapsuleButtonStyle: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 26, *) {
            content.buttonStyle(.glass)
        } else {
            content
                .buttonStyle(.bordered)
                .buttonBorderShape(.capsule)
                .tint(.primary)
        }
    }
}

/// The Foil switch, a capsule like the other buttons of the header, filled with the foil colour when
/// on: the cards read or added from the catalog next are recorded in foil.
private struct FoilButton: View {
    @Binding var isOn: Bool

    var body: some View {
        Button("Foil", systemImage: "sparkles") { isOn.toggle() }
            .font(.body.weight(.medium))
            .modifier(FoilButtonStyle(isOn: isOn))
            .accessibilityAddTraits(isOn ? .isSelected : [])
            .accessibilityHint("Les cartes ajoutées ensuite sont enregistrées en foil")
    }
}

private struct FoilButtonStyle: ViewModifier {
    let isOn: Bool

    func body(content: Content) -> some View {
        if isOn {
            if #available(iOS 26, *) {
                content.buttonStyle(.glassProminent).tint(Palette.secondary)
            } else {
                content
                    .buttonStyle(.borderedProminent)
                    .buttonBorderShape(.capsule)
                    .tint(Palette.secondary)
            }
        } else {
            content
                .foregroundStyle(.primary)
                .modifier(NeutralCapsuleButtonStyle())
        }
    }
}

/// The card of a line as a thumbnail when the platform has its image, the set symbol tinted by rarity
/// otherwise (card not in the catalog yet, no network, image not loaded).
private struct ScanBatchThumbnail: View {
    let line: ScanBatchLine

    var body: some View {
        LazyImage(url: CardArtwork.url(forSet: line.set, number: line.number)) { state in
            if let image = state.image {
                image.resizable().scaledToFill()
            } else {
                SetSymbol(setCode: line.set, size: 22)
                    .foregroundStyle(RarityColor.tint(forRarityCode: line.rarity))
            }
        }
        .frame(width: 36, height: 50)
        .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
        .accessibilityHidden(true)
    }
}

/// A line of the scan batch: its card, its finish, and the stepper of its quantity.
/// A tap on the card opens its corrections.
private struct ScanBatchRow: View {
    let line: ScanBatchLine
    let scanner: ScannerViewModel
    /// The catalog refused this card on the last validation.
    var isRejected = false
    /// The trend price of the line, quantity included; nil when hidden or unknown.
    let cents: Int?
    @Binding var quantity: Int
    /// Opens the catalog search that replaces the card.
    let onReplace: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            ScanLineMenu(line: line, scanner: scanner, onReplace: onReplace) {
                card
            }
            Stepper(value: $quantity, in: 0 ... ScanBatch.maxQuantity) {
                Text("\(quantity)")
                    .font(.body.monospacedDigit())
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .frame(width: 150)
            .accessibilityLabel("Quantité de \(line.displayName)")
        }
    }

    private var card: some View {
        HStack(spacing: 12) {
            ScanBatchThumbnail(line: line)
            VStack(alignment: .leading, spacing: 2) {
                Text(line.displayName)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)
                    .truncationMode(.tail)
                HStack(spacing: 6) {
                    PrintedLine(number: line.number, set: line.set, foil: line.foil, language: line.language,
                                style: .caption)
                        .foregroundStyle(.secondary)
                    if isRejected {
                        Label("Hors catalogue", systemImage: "exclamationmark.triangle.fill")
                            .font(.caption2.weight(.bold))
                            .foregroundStyle(.red)
                    }
                    if let cents {
                        Text(Price.euros(cents: cents))
                            .font(.caption.monospacedDigit().weight(.semibold))
                    }
                }
            }
        }
    }
}

/// The batch of the previews, kept in memory: nothing is written on the device.
private struct PreviewScanBatchStorage: ScanBatchStorage {
    let batch: ScanBatch

    /// A batch of the given cards, as the camera would have added them.
    init(cards: [RecognisedCard]) {
        var batch = ScanBatch()
        for card in cards {
            batch.addFromCamera(card, foil: false)
        }
        self.batch = batch
    }

    func load() -> ScanBatch? {
        batch
    }

    func save(_: ScanBatch) {}
}

/// Cards of the local catalog: the first two have an image in the dev environment (frontend on
/// localhost:3000), the last one shows the set symbol.
private let previewCards = [
    RecognisedCard(set: "BLB", number: "155", language: .english, rarity: "R",
                   name: "Sunspine Lynx", frenchName: nil),
    RecognisedCard(set: "BLB", number: "90", language: .english, rarity: "R",
                   name: "Darkstar Augur", frenchName: nil),
    RecognisedCard(set: "BLB", number: "18", language: .english, rarity: "R",
                   name: "Jackdaw Savior", frenchName: nil)
]

/// The drawer as the scanner shows it: a sheet over the camera, at the given size.
private struct ScannerDrawerPreview: View {
    @State private var detent: PresentationDetent
    @State private var scanner: ScannerViewModel
    @State private var validation: ScanValidationViewModel

    /// `lastRecognised`: the card the camera just read, shown by the folded drawer.
    init(detent: PresentationDetent, cards: [RecognisedCard] = [], lastRecognised: RecognisedCard? = nil) {
        let scanner = ScannerViewModel(
            index: ScanIndexModel(api: APIClientFactory.preview),
            storage: PreviewScanBatchStorage(cards: cards),
            lastRecognised: lastRecognised
        )
        _detent = State(initialValue: detent)
        _scanner = State(initialValue: scanner)
        _validation = State(initialValue: ScanValidationViewModel(scanner: scanner, api: APIClientFactory.preview))
    }

    var body: some View {
        Color.black
            .ignoresSafeArea()
            .sheet(isPresented: .constant(true)) {
                ScannerDrawer(scanner: scanner, validation: validation, detent: $detent) {}
            }
    }
}

#Preview("Replié, lot vide") {
    ScannerDrawerPreview(detent: .scannerCollapsed)
}

#Preview("Replié") {
    ScannerDrawerPreview(detent: .scannerCollapsed, cards: previewCards)
}

#Preview("Replié, carte scannée") {
    ScannerDrawerPreview(detent: .scannerCollapsed, cards: previewCards, lastRecognised: previewCards[0])
}

#Preview("Replié, carte sans image") {
    ScannerDrawerPreview(detent: .scannerCollapsed, cards: previewCards, lastRecognised: previewCards[2])
}

#Preview("Moyen, lot vide") {
    ScannerDrawerPreview(detent: .medium)
}

#Preview("Grand") {
    ScannerDrawerPreview(detent: .large, cards: previewCards)
}
