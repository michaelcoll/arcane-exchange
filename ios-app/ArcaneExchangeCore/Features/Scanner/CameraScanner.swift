import AVFoundation
import CoreImage
import Foundation
import Vision

/// Whether the camera can be used, as the Scanner tab needs to tell the player.
package enum CameraStatus: Equatable {
    case starting
    case running
    /// The player refused (or restricted) access: only the Settings app can change it.
    case denied
    /// No back camera (the simulator).
    case unavailable
}

/// Keeps the camera open and, for every frame it can keep up with, finds the card (`VNDetectRectanglesRequest`),
/// straightens the strip of its printed line (perspective correction) and runs Vision text recognition on it.
/// The text lines go to `onFrame`, on the capture queue.
///
/// Glue only, taken over from the prototype `prototypes/ocr-collector-line`: no unit test, the camera does
/// not exist in the simulator. Everything it feeds is tested (`CardRecogniser`, `ReadingStabiliser`,
/// `CardContour`, `ScanGuide`).
package final class CameraScanner: NSObject, @unchecked Sendable {
    package let session = AVCaptureSession()
    /// Receives the text lines Vision read in the card strip of one frame (empty when no card was
    /// found) and the frame's time in seconds. Set before `start()`.
    package var onFrame: (@Sendable ([String], TimeInterval) -> Void)?

    private let queue = DispatchQueue(label: "scanner.camera")
    private var configured = false

    override package init() {
        super.init()
    }

    /// Asks for access when the player was never asked, then starts the capture.
    package func start() async -> CameraStatus {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized: break
        case .notDetermined:
            guard await AVCaptureDevice.requestAccess(for: .video) else { return .denied }
        default: return .denied
        }
        return await withCheckedContinuation { continuation in
            queue.async { [self] in
                if !configured {
                    configured = configure()
                }
                guard configured else { return continuation.resume(returning: .unavailable) }
                if !session.isRunning {
                    session.startRunning()
                }
                continuation.resume(returning: .running)
            }
        }
    }

    package func stop() {
        queue.async { [session] in
            if session.isRunning {
                session.stopRunning()
            }
        }
    }

    /// The virtual camera first: it switches by itself to the ultra wide (macro) when the card is
    /// closer than the wide lens can focus. The wide lens alone where there is no virtual camera.
    private static func backCamera() -> AVCaptureDevice? {
        for type in [AVCaptureDevice.DeviceType.builtInTripleCamera, .builtInDualWideCamera, .builtInWideAngleCamera] {
            if let device = AVCaptureDevice.default(type, for: .video, position: .back) {
                return device
            }
        }
        return nil
    }

    private func configure() -> Bool {
        guard let device = Self.backCamera(), let input = try? AVCaptureDeviceInput(device: device) else {
            return false
        }
        session.beginConfiguration()
        // The printed line is tiny: ask for as many pixels as the device gives.
        session.sessionPreset = session.canSetSessionPreset(.hd4K3840x2160) ? .hd4K3840x2160 : .hd1920x1080
        if session.canAddInput(input) {
            session.addInput(input)
        }
        let output = AVCaptureVideoDataOutput()
        output.alwaysDiscardsLateVideoFrames = true // OCR is slower than the camera: drop, don't queue.
        output.setSampleBufferDelegate(self, queue: queue)
        if session.canAddOutput(output) {
            session.addOutput(output)
        }
        session.commitConfiguration()
        aimAtThePrintedLine(device)
        return true
    }

    /// Zooms so the card in the outline sits beyond the minimum focus distance, and focuses and
    /// exposes on the printed line (see `ScanGuide`).
    ///
    /// On a virtual camera, zoom ×1 is the ultra wide: the zoom is at least the switch-over to the
    /// wide lens (the "1×" of the Camera app), and the camera itself falls back to the ultra wide
    /// when the card is too close for the wide lens. Its minimum focus distance is the ultra wide's.
    private func aimAtThePrintedLine(_ device: AVCaptureDevice) {
        guard (try? device.lockForConfiguration()) != nil else { return }
        defer { device.unlockForConfiguration() }

        if device.isVirtualDevice {
            device.setPrimaryConstituentDeviceSwitchingBehavior(.auto, restrictedSwitchingBehaviorConditions: [])
        }
        let wideLens = device.virtualDeviceSwitchOverVideoZoomFactors.first.map { Double(truncating: $0) } ?? 1
        var zoom = wideLens
        let minimumFocus = device.minimumFocusDistance // millimetres, -1 when unknown
        if minimumFocus > 0 {
            zoom = max(zoom, ScanGuide.zoomFactor(
                horizontalFieldOfViewDegrees: Double(device.activeFormat.videoFieldOfView),
                minimumFocusDistanceMillimetres: Double(minimumFocus)
            ))
        }
        zoom = min(zoom, Double(device.activeFormat.videoMaxZoomFactor))
        device.videoZoomFactor = CGFloat(zoom)
        if device.isAutoFocusRangeRestrictionSupported {
            device.autoFocusRangeRestriction = .near
        }
        if device.isSmoothAutoFocusSupported {
            device.isSmoothAutoFocusEnabled = false
        }
        // The point of interest takes effect when the mode is set, so set it first.
        if device.isFocusPointOfInterestSupported {
            device.focusPointOfInterest = ScanGuide.focusPoint
        }
        if device.isFocusModeSupported(.continuousAutoFocus) {
            device.focusMode = .continuousAutoFocus
        }
        if device.isExposurePointOfInterestSupported {
            device.exposurePointOfInterest = ScanGuide.focusPoint
        }
        if device.isExposureModeSupported(.continuousAutoExposure) {
            device.exposureMode = .continuousAutoExposure
        }
    }
}

extension CameraScanner: AVCaptureVideoDataOutputSampleBufferDelegate {
    package func captureOutput(
        _: AVCaptureOutput,
        didOutput sampleBuffer: CMSampleBuffer,
        from _: AVCaptureConnection
    ) {
        guard let buffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
        let frameTime = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sampleBuffer))
        let lines = Self.cardContour(in: buffer).map { Self.linesInCollectorStrip(of: $0, in: buffer) } ?? []
        onFrame?(lines, frameTime)
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
        // Back camera buffers are landscape; the phone is held in portrait.
        try? VNImageRequestHandler(cvPixelBuffer: buffer, orientation: .right).perform([request])
        return (request.results ?? []).lazy.compactMap { rectangle in
            CardContour(visionTopLeft: rectangle.topLeft, topRight: rectangle.topRight,
                        bottomRight: rectangle.bottomRight, bottomLeft: rectangle.bottomLeft)
        }.first
    }

    /// Straightens the printed line strip of the card found (rotation and perspective) and reads
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
            "inputBottomRight": pixel(strip.bottomRight), "inputBottomLeft": pixel(strip.bottomLeft)
        ])
        let atOrigin = straightened.transformed(
            by: CGAffineTransform(translationX: -straightened.extent.minX, y: -straightened.extent.minY)
        )
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        try? VNImageRequestHandler(ciImage: atOrigin, orientation: .up).perform([request])
        return (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
    }
}
