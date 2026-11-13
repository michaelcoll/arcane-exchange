import Foundation

/// What the Scanner tab shows: the last card the camera recognised. It is fed the text lines of
/// every frame (`frame(lines:at:)`), by the camera in the app and by the tests; it knows nothing of
/// the camera itself.
///
/// The seam the scan batch grows from: a card retained by the stabiliser reaches `retain(_:)`, which
/// is where it will join the batch.
@MainActor
@Observable
package final class ScannerViewModel {
    /// The last card retained by the stabiliser.
    package private(set) var lastRecognised: RecognisedCard?
    /// Counts the retained cards, the same card shown again included: drives the haptic feedback.
    package private(set) var recognitionCount = 0

    package private(set) var cameraStatus = CameraStatus.starting
    /// The camera, for the preview layer.
    package let camera: CameraScanner

    private let index: ScanIndexModel
    private var stabiliser = ReadingStabiliser()

    package init(index: ScanIndexModel, camera: CameraScanner = CameraScanner()) {
        self.index = index
        self.camera = camera
    }

    /// Opens the camera (asking for access the first time) and feeds the frames in.
    package func start() async {
        camera.onFrame = { [weak self] lines, time in
            DispatchQueue.main.async {
                MainActor.assumeIsolated { self?.frame(lines: lines, at: time) }
            }
        }
        cameraStatus = await camera.start()
    }

    package func stop() {
        camera.stop()
    }

    package func frame(lines: [String], at time: TimeInterval) {
        let card = index.index.flatMap { CardRecogniser.recognise(lines: lines, in: $0) }
        if let retained = stabiliser.observe(card, hasText: !lines.isEmpty, at: time) {
            retain(retained)
        }
    }

    private func retain(_ card: RecognisedCard) {
        lastRecognised = card
        recognitionCount += 1
    }
}
