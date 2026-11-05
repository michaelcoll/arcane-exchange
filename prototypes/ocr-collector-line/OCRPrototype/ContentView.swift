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
                Text(latencyLabel).font(.headline.monospacedDigit())
                Spacer()
                Text("\(scanner.ocrMilliseconds) ms / frame").font(.caption.monospacedDigit())
            }
            .padding()
            Text(readingLabel).font(.title3.monospaced().bold())
            ScrollView {
                Text(scanner.lines.joined(separator: "\n"))
                    .font(.system(.body, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal)
            }
            HStack {
                Button("Copy reading") { UIPasteboard.general.string = fixture }
                ShareLink("Export", item: fixture)
            }
            .buttonStyle(.bordered)
            .disabled(scanner.lastReading.isEmpty)
            .padding()
        }
        .task { scanner.start() }
        .onDisappear { scanner.stop() }
    }

    private var latencyLabel: String {
        scanner.latency.map { String(format: "Read in %.2f s", $0) } ?? "No reading yet"
    }

    private var readingLabel: String {
        guard let reading = scanner.reading else { return "-" }
        let token = reading.isToken ? " (token)" : ""
        return "\(reading.set.uppercased()) #\(reading.number) \(reading.language)\(token)"
    }

    /// The raw text frozen at the measure, one Vision line per row.
    private var fixture: String { scanner.lastReading.joined(separator: "\n") }
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
