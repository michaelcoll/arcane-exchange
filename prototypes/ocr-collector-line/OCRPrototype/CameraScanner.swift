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
    /// Lines frozen when the latency was measured: the text to keep as a fixture.
    private(set) var lastReading: [String] = []
    private(set) var ocrMilliseconds: Int = 0
    private(set) var status = "Waiting for the camera"

    @ObservationIgnored let session = AVCaptureSession()
    @ObservationIgnored private let queue = DispatchQueue(label: "ocr.camera")
    @ObservationIgnored private var timer = PresentationTimer()
    @ObservationIgnored private var configured = false

    func start() {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized: run()
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { [weak self] granted in
                DispatchQueue.main.async { granted ? self?.run() : self?.fail("Camera access denied") }
            }
        default: fail("Camera access denied: enable it in Settings")
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
            DispatchQueue.main.async { self.fail("No camera available (the simulator has none)") }
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
        DispatchQueue.main.async { self.status = "Show a card" }
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
                self.lastReading = recognised
            }
        }
    }
}
