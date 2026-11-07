import AVFoundation
import SwiftUI

struct ContentView: View {
    @State private var scanner = CameraScanner()

    var body: some View {
        VStack(spacing: 0) {
            CameraPreview(session: scanner.session)
                .frame(maxHeight: 320)
                .overlay { Text(scanner.status).foregroundStyle(.secondary) }
            HStack {
                Text(ScanLabels.latency(scanner.latency)).font(.headline.monospacedDigit())
                Spacer()
                Text("\(scanner.ocrMilliseconds) ms / image").font(.caption.monospacedDigit())
            }
            .padding()
            Text(ScanLabels.reading(scanner.reading)).font(.title3.monospaced().bold())
            Text(ScanLabels.lookup(scanner.lookup))
                .font(.callout)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal)
            ScrollView {
                Text(scanner.lines.joined(separator: "\n"))
                    .font(.system(.body, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal)
            }
            HStack {
                Button("Copier le texte brut") { UIPasteboard.general.string = fixtureLines }
                ShareLink("Exporter", item: fixtureLines)
            }
            .buttonStyle(.bordered)
            .disabled(scanner.lines.isEmpty)
            .padding()
        }
        .task { scanner.start() }
        .onDisappear { scanner.stop() }
    }

    /// The raw text of the current image, one Vision line per row, available even when no reading
    /// was retained (a failed read is the fixture worth keeping).
    private var fixtureLines: String { scanner.lines.joined(separator: "\n") }
}

private struct CameraPreview: UIViewRepresentable {
    let session: AVCaptureSession

    func makeUIView(context: Context) -> PreviewView {
        let view = PreviewView()
        view.previewLayer.session = session
        view.previewLayer.videoGravity = .resizeAspectFill
        return view
    }

    func updateUIView(_ uiView: PreviewView, context: Context) {}
}

private final class PreviewView: UIView {
    override class var layerClass: AnyClass { AVCaptureVideoPreviewLayer.self }
    var previewLayer: AVCaptureVideoPreviewLayer { layer as! AVCaptureVideoPreviewLayer }
}
