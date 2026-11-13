import AVFoundation
import SwiftUI

/// The camera image. It shows the whole 16:9 frame (`resizeAspect`), so the outline of `ScanGuide`
/// drawn over it lines up with what Vision sees.
struct CameraPreview: UIViewRepresentable {
    let session: AVCaptureSession

    func makeUIView(context _: Context) -> PreviewView {
        let view = PreviewView()
        view.previewLayer.session = session
        view.previewLayer.videoGravity = .resizeAspect
        return view
    }

    func updateUIView(_: PreviewView, context _: Context) {}

    class PreviewView: UIView {
        override class var layerClass: AnyClass {
            AVCaptureVideoPreviewLayer.self
        }

        // swiftlint:disable force_cast
        var previewLayer: AVCaptureVideoPreviewLayer {
            layer as! AVCaptureVideoPreviewLayer
        }
        // swiftlint:enable force_cast
    }
}
