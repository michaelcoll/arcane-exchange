import AVFoundation
import SwiftUI

struct ContentView: View {
    @State private var scanner = CameraScanner()

    @State private var showsRawText = false

    var body: some View {
        ZStack(alignment: .bottom) {
            Color.black.ignoresSafeArea()
            // The whole 9:16 image across the full width, from the top edge, so the outline and
            // Vision's region share `ScanGuide` fractions.
            VStack(spacing: 0) {
                CameraPreview(session: scanner.session)
                    .aspectRatio(ScanGuide.imageAspect, contentMode: .fit)
                    .overlay { ScanGuideOverlay(mode: scanner.mode, contour: scanner.contour) }
                Spacer(minLength: 0)
            }
            .ignoresSafeArea(edges: .top)
            resultsPanel
        }
        .sensoryFeedback(.success, trigger: scanner.retainedCount)
        .task { scanner.start() }
        .onDisappear { scanner.stop() }
    }

    /// Compact results over the bottom of the image, below the outline.
    private var resultsPanel: some View {
        VStack(alignment: .leading, spacing: 6) {
            Picker("Mode", selection: $scanner.mode) {
                ForEach(ScanMode.allCases) { Text(ScanLabels.mode($0)).tag($0) }
            }
            .pickerStyle(.segmented)
            if let detection = ScanLabels.contour(scanner.contour, in: scanner.mode) {
                Label(detection, systemImage: scanner.contour == nil ? "viewfinder" : "checkmark.rectangle.fill")
                    .font(.subheadline.bold())
                    .foregroundStyle(scanner.contour == nil ? Color.secondary : Color.green)
            }
            HStack(alignment: .firstTextBaseline) {
                Text(ScanLabels.reading(scanner.reading)).font(.headline.monospaced())
                Spacer()
                Text(ScanLabels.latency(scanner.latency)).font(.subheadline.monospacedDigit())
            }
            Text(ScanLabels.lookup(scanner.lookup)).font(.footnote).lineLimit(4)
            Text("\(scanner.status) · \(scanner.ocrMilliseconds) ms / image")
                .font(.caption2.monospacedDigit())
                .foregroundStyle(.secondary)
            Text("Objectif : \(scanner.lens) · \(scanner.optics)")
                .font(.caption2.monospacedDigit())
                .foregroundStyle(.secondary)
            HStack {
                Button(showsRawText ? "Masquer le texte brut" : "Texte brut") { showsRawText.toggle() }
                Spacer()
                Button("Copier") { UIPasteboard.general.string = fixtureLines }
                ShareLink("Exporter", item: fixtureLines)
            }
            .font(.footnote)
            .buttonStyle(.bordered)
            .disabled(scanner.lines.isEmpty)
            if showsRawText {
                ScrollView {
                    Text(fixtureLines)
                        .font(.system(.caption, design: .monospaced))
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 90)
            }
        }
        .padding(12)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
        .padding(.horizontal, 8)
    }

    /// The raw text of the current image, one Vision line per row, available even when no reading
    /// was retained (a failed read is the fixture worth keeping).
    private var fixtureLines: String { scanner.lines.joined(separator: "\n") }
}

/// The card outline and, in yellow, the strip Vision reads (Vision's origin is bottom-left). In
/// the contour mode, the outline is only a distance guide: the card found is drawn in green and
/// the strip read follows it.
private struct ScanGuideOverlay: View {
    let mode: ScanMode
    let contour: CardContour?

    var body: some View {
        GeometryReader { geometry in
            let size = geometry.size
            let card = ScanGuide.card
            RoundedRectangle(cornerRadius: 8)
                .stroke(.white.opacity(mode == .fixedStrip ? 1 : 0.35), lineWidth: 2)
                .frame(width: card.width * size.width, height: card.height * size.height)
                .position(x: card.midX * size.width, y: card.midY * size.height)
            switch mode {
            case .fixedStrip:
                let roi = ScanGuide.regionOfInterest
                Rectangle()
                    .stroke(.yellow, style: StrokeStyle(lineWidth: 1, dash: [4]))
                    .frame(width: roi.width * size.width, height: roi.height * size.height)
                    .position(x: roi.midX * size.width, y: (1 - roi.midY) * size.height)
            case .cardContour:
                if let contour {
                    quadrilateral(contour, in: size)
                        .fill(.green.opacity(0.15))
                        .stroke(.green, lineWidth: 4)
                    quadrilateral(contour.collectorStrip, in: size)
                        .stroke(.yellow, style: StrokeStyle(lineWidth: 1, dash: [4]))
                }
            }
        }
        .allowsHitTesting(false)
    }

    private func quadrilateral(_ contour: CardContour, in size: CGSize) -> Path {
        Path { path in
            path.addLines(contour.corners.map { CGPoint(x: $0.x * size.width, y: $0.y * size.height) })
            path.closeSubpath()
        }
    }
}

private struct CameraPreview: UIViewRepresentable {
    let session: AVCaptureSession

    func makeUIView(context: Context) -> PreviewView {
        let view = PreviewView()
        view.previewLayer.session = session
        view.previewLayer.videoGravity = .resizeAspect
        return view
    }

    func updateUIView(_ uiView: PreviewView, context: Context) {}
}

private final class PreviewView: UIView {
    override class var layerClass: AnyClass { AVCaptureVideoPreviewLayer.self }
    var previewLayer: AVCaptureVideoPreviewLayer { layer as! AVCaptureVideoPreviewLayer }
}
