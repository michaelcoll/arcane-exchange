import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The Scanner tab: the camera, full screen, and the scan batch it fills, shown in the drawer.
struct ScannerView: View {
    @State private var scanner: ScannerViewModel
    @State private var validation: ScanValidationViewModel
    @State private var isDrawerOpen = false
    @State private var isConfirmingClear = false
    @State private var isValidating = false
    private let index: ScanIndexModel

    init(index: ScanIndexModel, api: any APIProtocol) {
        self.index = index
        let scanner = ScannerViewModel(
            index: index,
            storage: DeviceScanBatchStorage(),
            prices: ScanBatchPrices(api: api)
        )
        _scanner = State(initialValue: scanner)
        _validation = State(initialValue: ScanValidationViewModel(scanner: scanner, api: api))
    }

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            content
        }
        // On every opening of the tab, not only at launch.
        .task { await index.refresh() }
        // Restarts when the index becomes ready, and stops with the tab.
        .task(id: index.state) {
            if index.state == .ready {
                await scanner.start()
            }
        }
        .onDisappear { scanner.stop() }
        // On opening and whenever a card joins the batch (scanned, replaced or picked from the
        // catalog), not on a quantity change; no network just hides the prices.
        .task(id: scanner.batch.lines.map { "\($0.set)-\($0.number)" }) {
            await scanner.refreshPrices()
        }
        .sensoryFeedback(.success, trigger: scanner.recognitionCount)
        // Nothing is added while the list or the confirmation is open.
        .onChange(of: isDrawerOpen || isConfirmingClear || isValidating) { _, paused in scanner.isPaused = paused }
        .sheet(isPresented: $isValidating) {
            ScanValidationSheet(
                model: validation,
                cardCount: scanner.batch.cardCount,
                valueCents: scanner.batchValue?.cents
            ) {
                // Back to the camera.
                withAnimation(.snappy) { isDrawerOpen = false }
            } onRefused: {
                // The refused lines are flagged in the list.
                withAnimation(.snappy) { isDrawerOpen = true }
            }
        }
        .alert("Vider le lot ?", isPresented: $isConfirmingClear) {
            Button("Annuler", role: .cancel) {}
            Button("Vider", role: .destructive) { scanner.clear() }
        } message: {
            Text("Les cartes scannées seront retirées du lot. Ta collection n'est pas modifiée.")
        }
    }

    /// The Foil switch, and the toast offering to undo the card just added.
    private var topBar: some View {
        VStack(spacing: 10) {
            HStack {
                Spacer()
                Toggle(isOn: $scanner.foil) {
                    Label("Foil", systemImage: "sparkles")
                }
                .toggleStyle(.button)
                .tint(Palette.secondary)
                .background(.regularMaterial, in: Capsule())
                .accessibilityHint("Les cartes lues ensuite sont enregistrées en foil")
            }
            if let card = scanner.undoOffer {
                UndoToast(card: card) { scanner.undoLastAdd() }
                    .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .padding()
        .animation(.snappy, value: scanner.undoOffer)
        // A few seconds after each addition, the offer goes away.
        .task(id: scanner.recognitionCount) {
            try? await Task.sleep(for: .seconds(3))
            if !Task.isCancelled {
                scanner.dismissUndoOffer()
            }
        }
    }

    /// The drawer is always there: whatever the camera does, the batch stays to correct, validate or empty.
    private var content: some View {
        ZStack(alignment: .bottom) {
            background
            if isDrawerOpen {
                Color.black.opacity(0.4)
                    .ignoresSafeArea()
                    .onTapGesture { withAnimation(.snappy) { isDrawerOpen = false } }
            }
            ScannerDrawer(
                scanner: scanner,
                isOpen: $isDrawerOpen,
                rejectedLineIDs: validation.rejectedLineIDs,
                onClear: { isConfirmingClear = true },
                onFinish: { isValidating = true }
            )
        }
        .overlay(alignment: .top) {
            if isCameraRunning {
                topBar
            }
        }
    }

    private var isCameraRunning: Bool {
        index.state == .ready && (scanner.cameraStatus == .starting || scanner.cameraStatus == .running)
    }

    /// What fills the screen behind the drawer: the camera, or why there is none.
    @ViewBuilder private var background: some View {
        switch index.state {
        case .loading:
            ProgressView().tint(.white)
        case .needsNetwork:
            ContentUnavailableView(
                "Index de scan indisponible",
                systemImage: "wifi.slash",
                description: Text(
                    "Le scanner a besoin du réseau une première fois, pour télécharger la liste des cartes. "
                        + "Ensuite, il fonctionne sans."
                )
            )
            .environment(\.colorScheme, .dark)
        case .ready:
            camera
        }
    }

    @ViewBuilder private var camera: some View {
        switch scanner.cameraStatus {
        case .starting, .running:
            ZStack {
                CameraPreview(session: scanner.camera.session)
                    .ignoresSafeArea()
                CardOutline()
            }
        case .denied:
            ContentUnavailableView {
                Label("Caméra refusée", systemImage: "camera.fill")
            } description: {
                Text("Autorise la caméra dans les Réglages pour scanner tes cartes.")
            } actions: {
                Button("Ouvrir les Réglages") {
                    if let url = URL(string: UIApplication.openSettingsURLString) {
                        UIApplication.shared.open(url)
                    }
                }
                .buttonStyle(.borderedProminent)
            }
            .environment(\.colorScheme, .dark)
        case .unavailable:
            ContentUnavailableView(
                "Aucune caméra",
                systemImage: "camera.metering.unknown",
                description: Text("Cet appareil n'a pas de caméra arrière, le scanner ne peut pas fonctionner.")
            )
            .environment(\.colorScheme, .dark)
        }
    }
}

/// The toast that follows each addition: the card added, and the way to take it off.
private struct UndoToast: View {
    let card: RecognisedCard
    let onUndo: () -> Void

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "checkmark.circle.fill")
                .foregroundStyle(Palette.primary)
            Text(card.displayName)
                .font(.subheadline.weight(.semibold))
                .lineLimit(1)
            Spacer(minLength: 0)
            Button("Annuler", action: onUndo)
                .font(.subheadline.weight(.semibold))
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(.regularMaterial, in: Capsule())
    }
}

/// The outline of a card over the camera image, where `ScanGuide` expects it.
private struct CardOutline: View {
    var body: some View {
        GeometryReader { geometry in
            // The preview shows the whole 16:9 portrait image, centred in the screen.
            let image = fittedImage(in: geometry.size)
            let card = ScanGuide.card
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(.white.opacity(0.7), style: StrokeStyle(lineWidth: 2, dash: [10, 6]))
                .frame(width: card.width * image.width, height: card.height * image.height)
                .position(x: image.minX + card.midX * image.width, y: image.minY + card.midY * image.height)
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }

    private func fittedImage(in size: CGSize) -> CGRect {
        let width = min(size.width, size.height * ScanGuide.imageAspect)
        let height = width / ScanGuide.imageAspect
        return CGRect(x: (size.width - width) / 2, y: (size.height - height) / 2, width: width, height: height)
    }
}

#Preview {
    ScannerView(index: ScanIndexModel(api: APIClientFactory.preview), api: APIClientFactory.preview)
}
