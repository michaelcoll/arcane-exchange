import ArcaneExchangeCore
import SwiftUI

/// The Scanner tab: the camera, full screen, and the card it recognises. Nothing is added to a
/// scan batch yet.
struct ScannerView: View {
    @State private var scanner: ScannerViewModel
    private let index: ScanIndexModel

    init(index: ScanIndexModel) {
        self.index = index
        _scanner = State(initialValue: ScannerViewModel(index: index))
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
        .sensoryFeedback(.success, trigger: scanner.recognitionCount)
    }

    @ViewBuilder private var content: some View {
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
            ZStack(alignment: .bottom) {
                CameraPreview(session: scanner.camera.session)
                    .ignoresSafeArea()
                CardOutline()
                ScannerResultCard(card: scanner.lastRecognised)
                    .padding()
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
    ScannerView(index: ScanIndexModel(api: APIClientFactory.preview))
}
