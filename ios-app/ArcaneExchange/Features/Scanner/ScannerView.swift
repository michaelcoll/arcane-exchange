import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The Scanner tab: the camera, full screen without the tab bar, and the scan batch it fills, shown
/// in the drawer. The close button goes back to the Collection tab.
struct ScannerView: View {
    @Environment(AppRouter.self) private var router
    @State private var scanner: ScannerViewModel
    @State private var validation: ScanValidationViewModel
    @State private var drawerDetent: PresentationDetent = .scannerCollapsed
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
            background
                // What the camera does not show stays clear of the folded drawer.
                .safeAreaPadding(.bottom, ScannerDrawer.collapsedHeight)
        }
        .overlay(alignment: .top) { topBar }
        .toolbar(.hidden, for: .tabBar)
        // The drawer is always there on this tab: whatever the camera does, the batch stays to
        // correct, validate or empty. It goes with the tab.
        .sheet(isPresented: .constant(router.tab == .scanner)) {
            ScannerDrawer(scanner: scanner, validation: validation, detent: $drawerDetent) {
                router.tab = .collection
            }
        }
        // On every opening of the tab, not only at launch.
        .task { await index.refresh() }
        // Restarts when the index becomes ready, and stops with the tab. The drawer pulled up to its
        // full height hides the camera: it stops, to save the battery.
        .task(id: CameraDemand(isIndexReady: index.state == .ready, isHidden: drawerDetent == .large)) {
            if index.state == .ready, drawerDetent != .large {
                await scanner.start()
            } else {
                scanner.stop()
            }
        }
        .onDisappear { scanner.stop() }
        // On opening and whenever a card joins the batch (scanned, replaced or picked from the
        // catalog), not on a quantity change; no network just hides the prices.
        .task(id: scanner.batch.lines.map { "\($0.set)-\($0.number)" }) {
            await scanner.refreshPrices()
        }
        .sensoryFeedback(.success, trigger: scanner.recognitionCount)
    }

    /// The toast offering to undo the card just added.
    private var topBar: some View {
        VStack(spacing: 10) {
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
                // Over the same area as the image, or the contour drifts from the card.
                DetectedCardOutline(card: scanner.detectedCard)
                    .ignoresSafeArea()
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

/// What decides whether the camera runs: the scan index, and the drawer covering it.
private struct CameraDemand: Equatable {
    let isIndexReady: Bool
    let isHidden: Bool
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

/// The card the camera finds, drawn in the app's blue over the camera image: the player sees
/// whether the card is picked up. Laid over the same area as `CameraPreview`.
private struct DetectedCardOutline: View {
    let card: CardContour?

    var body: some View {
        GeometryReader { geometry in
            if let card {
                // The preview shows the whole 16:9 portrait image, centred in its area.
                let outline = Path { path in
                    path.addLines(card.corners(in: ScanGuide.fittedImage(in: geometry.size)))
                    path.closeSubpath()
                }
                outline.fill(Palette.primary.opacity(0.15))
                outline.stroke(Palette.primary, style: StrokeStyle(lineWidth: 4, lineJoin: .round))
            }
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}

#Preview {
    // The drawer only shows on the Scanner tab.
    let router = AppRouter()
    router.tab = .scanner
    return ScannerView(index: ScanIndexModel(api: APIClientFactory.preview), api: APIClientFactory.preview)
        .environment(router)
}
