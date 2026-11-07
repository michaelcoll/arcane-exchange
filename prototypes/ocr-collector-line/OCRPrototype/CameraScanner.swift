import AVFoundation
import CoreImage
import Foundation
import Observation
import Vision

/// Keeps the camera open and runs Vision text recognition on every frame it can keep up with.
/// Glue only: no unit tests, the camera does not exist in the simulator.
@Observable
final class CameraScanner: NSObject {
    /// Raw recognised lines of the latest frame, in Vision's order.
    private(set) var lines: [String] = []
    /// Seconds between the card being presented and a stable reading (see `PresentationTimer`).
    private(set) var latency: TimeInterval?
    /// The retained collector line reading (complete, seen on two consecutive frames).
    private(set) var reading: CollectorReading?
    /// Scryfall lookup of the retained reading (nil before the first one).
    private(set) var lookup: LookupState?
    private(set) var ocrMilliseconds: Int = 0
    private(set) var status = "En attente de la caméra"
    /// Zoom and minimum focus distance, set once the camera is configured.
    private(set) var optics = ""
    /// The physical lens in use (the virtual camera switches between them by itself).
    private(set) var lens = ""
    /// The card found in the latest frame, in the contour mode only.
    private(set) var contour: CardContour?
    /// Counts the retained readings, the same card shown again included (drives the haptic).
    private(set) var retainedCount = 0
    /// Switching restarts the measure, so each mode's latency stands on its own.
    var mode: ScanMode = .fixedStrip {
        didSet {
            let mode = mode
            contour = nil
            latency = nil
            queue.async { [self] in
                captureMode = mode
                timer = Self.newTimer()
            }
        }
    }

    @ObservationIgnored let session = AVCaptureSession()
    @ObservationIgnored private let queue = DispatchQueue(label: "ocr.camera")
    @ObservationIgnored private var timer = newTimer()
    /// `mode` as seen by the capture queue, which must not read the observed property.
    @ObservationIgnored private var captureMode = ScanMode.fixedStrip
    @ObservationIgnored private var configured = false
    @ObservationIgnored private var lensObservation: NSKeyValueObservation?
    @ObservationIgnored private let client = ScryfallClient()
    @ObservationIgnored private var lookedUp: CollectorReading?

    private static func newTimer() -> PresentationTimer {
        // Vision only reads the collector line strip: one line is enough to call the card presented.
        var timer = PresentationTimer()
        timer.minLines = 1
        return timer
    }

    func start() {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized: run()
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { [weak self] granted in
                DispatchQueue.main.async { granted ? self?.run() : self?.fail("Accès à la caméra refusé") }
            }
        default: fail("Accès à la caméra refusé : l'autoriser dans les Réglages")
        }
    }

    func stop() {
        queue.async { [session] in if session.isRunning { session.stopRunning() } }
    }

    private func fail(_ message: String) { status = message }

    private func run() {
        queue.async { [self] in
            if !configured { configure() }
            if configured, !session.isRunning { session.startRunning() }
        }
    }

    /// The virtual camera first: it switches by itself to the ultra wide (macro) when the card is
    /// closer than the wide lens can focus. The wide lens alone where there is no virtual camera.
    private static func backCamera() -> AVCaptureDevice? {
        for type in [AVCaptureDevice.DeviceType.builtInTripleCamera, .builtInDualWideCamera, .builtInWideAngleCamera] {
            if let device = AVCaptureDevice.default(type, for: .video, position: .back) { return device }
        }
        return nil
    }

    private func configure() {
        guard let device = Self.backCamera(),
              let input = try? AVCaptureDeviceInput(device: device) else {
            DispatchQueue.main.async { self.fail("Aucune caméra disponible (le simulateur n'en a pas)") }
            return
        }
        session.beginConfiguration()
        // The collector line is tiny: ask for as many pixels as the device gives.
        session.sessionPreset = session.canSetSessionPreset(.hd4K3840x2160) ? .hd4K3840x2160 : .hd1920x1080
        if session.canAddInput(input) { session.addInput(input) }
        let output = AVCaptureVideoDataOutput()
        output.alwaysDiscardsLateVideoFrames = true  // OCR is slower than the camera: drop, don't queue.
        output.setSampleBufferDelegate(self, queue: queue)
        if session.canAddOutput(output) { session.addOutput(output) }
        session.commitConfiguration()

        let optics = aimAtTheCollectorLine(device)
        configured = true
        watchTheActiveLens(of: device)
        DispatchQueue.main.async {
            self.optics = optics
            self.status = "Placer la carte dans le cadre"
        }
    }

    /// Shows which physical lens the virtual camera is using, to see the macro switch happen.
    private func watchTheActiveLens(of device: AVCaptureDevice) {
        guard device.isVirtualDevice else {
            DispatchQueue.main.async { self.lens = Self.lensName(device.deviceType) }
            return
        }
        lensObservation = device.observe(\.activePrimaryConstituent, options: [.initial, .new]) { [weak self] device, _ in
            let name = device.activePrimaryConstituent.map { Self.lensName($0.deviceType) } ?? "?"
            DispatchQueue.main.async { self?.lens = name }
        }
    }

    private static func lensName(_ type: AVCaptureDevice.DeviceType) -> String {
        switch type {
        case .builtInUltraWideCamera: "ultra grand-angle (macro)"
        case .builtInWideAngleCamera: "grand-angle"
        case .builtInTelephotoCamera: "téléobjectif"
        default: type.rawValue
        }
    }

    /// Zooms so the card in the outline sits beyond the minimum focus distance, and focuses and
    /// exposes on the collector line (see `ScanGuide`). Returns a summary for the screen.
    ///
    /// On a virtual camera, zoom ×1 is the ultra wide: the zoom is at least the switch-over to the
    /// wide lens (the "1×" of the Camera app), and the camera itself falls back to the ultra wide
    /// when the card is too close for the wide lens. Its minimum focus distance is the ultra wide's.
    private func aimAtTheCollectorLine(_ device: AVCaptureDevice) -> String {
        guard (try? device.lockForConfiguration()) != nil else { return "réglages caméra refusés" }
        defer { device.unlockForConfiguration() }

        if device.isVirtualDevice {
            device.setPrimaryConstituentDeviceSwitchingBehavior(.auto, restrictedSwitchingBehaviorConditions: [])
        }
        let wideLens = device.virtualDeviceSwitchOverVideoZoomFactors.first.map { Double(truncating: $0) } ?? 1
        var zoom = wideLens
        let minimumFocus = device.minimumFocusDistance  // millimetres, -1 when unknown
        if minimumFocus > 0 {
            zoom = max(zoom, ScanGuide.zoomFactor(
                horizontalFieldOfViewDegrees: Double(device.activeFormat.videoFieldOfView),
                minimumFocusDistanceMillimetres: Double(minimumFocus)))
        }
        zoom = min(zoom, Double(device.activeFormat.videoMaxZoomFactor))
        device.videoZoomFactor = CGFloat(zoom)
        if device.isAutoFocusRangeRestrictionSupported { device.autoFocusRangeRestriction = .near }
        if device.isSmoothAutoFocusSupported { device.isSmoothAutoFocusEnabled = false }
        // The point of interest takes effect when the mode is set, so set it first.
        if device.isFocusPointOfInterestSupported { device.focusPointOfInterest = ScanGuide.focusPoint }
        if device.isFocusModeSupported(.continuousAutoFocus) { device.focusMode = .continuousAutoFocus }
        if device.isExposurePointOfInterestSupported { device.exposurePointOfInterest = ScanGuide.focusPoint }
        if device.isExposureModeSupported(.continuousAutoExposure) { device.exposureMode = .continuousAutoExposure }

        let focus = minimumFocus > 0 ? "netteté dès \(minimumFocus) mm" : "netteté minimale inconnue"
        return String(format: "zoom ×%.1f (×%.1f de l'app Appareil photo) · ", zoom, zoom / wideLens) + focus
    }
}

extension CameraScanner: AVCaptureVideoDataOutputSampleBufferDelegate {
    func captureOutput(_ output: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer,
                       from connection: AVCaptureConnection) {
        guard let buffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
        let frameTime = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sampleBuffer))

        let started = CFAbsoluteTimeGetCurrent()
        let mode = captureMode
        var contour: CardContour?
        let recognised: [String]
        switch mode {
        case .fixedStrip:
            recognised = Self.linesInFixedStrip(of: buffer)
        case .cardContour:
            contour = Self.cardContour(in: buffer)
            recognised = contour.map { Self.linesInCollectorStrip(of: $0, in: buffer) } ?? []
        }
        let elapsed = Int((CFAbsoluteTimeGetCurrent() - started) * 1000)

        let reading = CollectorLineReader.read(lines: recognised)
        let measured = timer.observe(lines: recognised, reading: reading, at: frameTime)
        DispatchQueue.main.async {
            // A frame still in flight when the mode changed belongs to the old mode.
            guard self.mode == mode else { return }
            self.lines = recognised
            self.ocrMilliseconds = elapsed
            self.contour = contour
            if let measured, let reading {
                self.latency = measured
                self.reading = reading
                self.retainedCount += 1
                self.lookUp(reading)
            }
        }
    }

    private static func textRequest() -> VNRecognizeTextRequest {
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        return request
    }

    private static func lines(of request: VNRecognizeTextRequest) -> [String] {
        (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
    }

    /// Only the bottom strip of the card outline: faster, and no rules text in the lines.
    private static func linesInFixedStrip(of buffer: CVPixelBuffer) -> [String] {
        let request = textRequest()
        request.regionOfInterest = ScanGuide.regionOfInterest
        // Back camera buffers are landscape; the phone is held in portrait.
        try? VNImageRequestHandler(cvPixelBuffer: buffer, orientation: .right).perform([request])
        return lines(of: request)
    }

    /// The card's edges: a portrait rectangle about a card's shape (63 / 88 = 0.72, widened for
    /// the perspective), taking at least a third of the image width.
    private static func cardContour(in buffer: CVPixelBuffer) -> CardContour? {
        let request = VNDetectRectanglesRequest()
        request.minimumAspectRatio = 0.55
        request.maximumAspectRatio = 0.9
        request.minimumSize = 0.33
        request.minimumConfidence = 0.5
        request.maximumObservations = 3
        try? VNImageRequestHandler(cvPixelBuffer: buffer, orientation: .right).perform([request])
        return (request.results ?? []).lazy.compactMap { rectangle in
            CardContour(visionTopLeft: rectangle.topLeft, topRight: rectangle.topRight,
                        bottomRight: rectangle.bottomRight, bottomLeft: rectangle.bottomLeft)
        }.first
    }

    /// Straightens the collector line strip of the card found (rotation and perspective) and reads
    /// all of it.
    private static func linesInCollectorStrip(of contour: CardContour, in buffer: CVPixelBuffer) -> [String] {
        let image = CIImage(cvPixelBuffer: buffer).oriented(.right)
        let extent = image.extent
        // Core Image's origin is bottom-left, the contour's top-left.
        func pixel(_ point: CGPoint) -> CIVector {
            CIVector(x: extent.minX + point.x * extent.width, y: extent.minY + (1 - point.y) * extent.height)
        }
        let strip = contour.collectorStrip
        let straightened = image.applyingFilter("CIPerspectiveCorrection", parameters: [
            "inputTopLeft": pixel(strip.topLeft), "inputTopRight": pixel(strip.topRight),
            "inputBottomRight": pixel(strip.bottomRight), "inputBottomLeft": pixel(strip.bottomLeft),
        ])
        let atOrigin = straightened.transformed(
            by: CGAffineTransform(translationX: -straightened.extent.minX, y: -straightened.extent.minY))
        let request = textRequest()
        try? VNImageRequestHandler(ciImage: atOrigin, orientation: .up).perform([request])
        return lines(of: request)
    }

    /// One request per distinct retained reading (the client also caches per set/number/language).
    private func lookUp(_ reading: CollectorReading) {
        guard reading != lookedUp else { return }
        lookedUp = reading
        lookup = .loading
        Task {
            let outcome = await client.lookup(reading)
            if lookedUp == reading { lookup = outcome }
        }
    }
}
