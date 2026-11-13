import Foundation

/// What the Scanner tab shows: the last card the camera recognised and the scan batch it feeds. It
/// is fed the text lines of every frame (`frame(lines:at:)`), by the camera in the app and by the
/// tests; it knows nothing of the camera itself.
///
/// A card retained by the stabiliser reaches `retain(_:)`, the one place it joins the batch. The
/// batch is a pure `ScanBatch`, saved through the injected storage at every change.
@MainActor
@Observable
package final class ScannerViewModel {
    /// The last card retained by the stabiliser, whether the batch took it or not.
    package private(set) var lastRecognised: RecognisedCard?
    /// Counts the cards the batch took: drives the haptic feedback.
    package private(set) var recognitionCount = 0

    package private(set) var batch: ScanBatch {
        didSet { storage.save(batch) }
    }

    /// The Foil switch: the finish of the cards read from now on.
    package var foil = false
    /// While true, the frames are ignored: the list or a confirmation is open.
    package var isPaused = false
    /// The card the camera just added, as long as the toast offers to undo it. The view withdraws
    /// the offer after a few seconds (`dismissUndoOffer()`).
    package private(set) var undoOffer: RecognisedCard?

    package private(set) var cameraStatus = CameraStatus.starting
    /// The camera, for the preview layer.
    package let camera: CameraScanner

    private let index: ScanIndexModel
    private var stabiliser = ReadingStabiliser()

    private let storage: any ScanBatchStorage

    package init(index: ScanIndexModel, storage: any ScanBatchStorage, camera: CameraScanner = CameraScanner()) {
        self.index = index
        self.storage = storage
        self.camera = camera
        batch = storage.load() ?? ScanBatch()
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
        guard !isPaused else { return }
        let card = index.index.flatMap { CardRecogniser.recognise(lines: lines, in: $0) }
        if let retained = stabiliser.observe(card, hasText: !lines.isEmpty, at: time) {
            retain(retained)
        }
    }

    private func retain(_ card: RecognisedCard) {
        lastRecognised = card
        if batch.addFromCamera(card, foil: foil) {
            undoOffer = card
            recognitionCount += 1
        }
    }

    /// Takes off the copy the camera just added.
    package func undoLastAdd() {
        guard undoOffer != nil else { return }
        batch.undoLastCameraAdd()
        undoOffer = nil
    }

    package func dismissUndoOffer() {
        undoOffer = nil
    }

    /// Sets the quantity of a line, 0 removing it.
    package func setQuantity(_ quantity: Int, ofLine id: String) {
        batch.setQuantity(quantity, ofLine: id)
        undoOffer = nil
    }

    /// Empties the batch, which gets a new identifier.
    package func clear() {
        batch.clear()
        undoOffer = nil
    }
}
