import AVFoundation
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

    @ObservationIgnored let session = AVCaptureSession()
    @ObservationIgnored private let queue = DispatchQueue(label: "ocr.camera")
    @ObservationIgnored private var timer = PresentationTimer()
    @ObservationIgnored private var configured = false
    @ObservationIgnored private let client = ScryfallClient()
    @ObservationIgnored private var lookedUp: CollectorReading?

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

    private func configure() {
        guard let device = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back),
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

        if (try? device.lockForConfiguration()) != nil {
            if device.isFocusModeSupported(.continuousAutoFocus) { device.focusMode = .continuousAutoFocus }
            device.unlockForConfiguration()
        }
        configured = true
        DispatchQueue.main.async { self.status = "Présenter une carte" }
    }
}

extension CameraScanner: AVCaptureVideoDataOutputSampleBufferDelegate {
    func captureOutput(_ output: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer,
                       from connection: AVCaptureConnection) {
        guard let buffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
        let frameTime = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sampleBuffer))

        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        // Back camera buffers are landscape; the phone is held in portrait.
        let handler = VNImageRequestHandler(cvPixelBuffer: buffer, orientation: .right)
        let started = CFAbsoluteTimeGetCurrent()
        try? handler.perform([request])
        let elapsed = Int((CFAbsoluteTimeGetCurrent() - started) * 1000)

        let recognised = (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
        let reading = CollectorLineReader.read(lines: recognised)
        let measured = timer.observe(lines: recognised, reading: reading, at: frameTime)
        DispatchQueue.main.async {
            self.lines = recognised
            self.ocrMilliseconds = elapsed
            if let measured, let reading {
                self.latency = measured
                self.reading = reading
                self.lookUp(reading)
            }
        }
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
