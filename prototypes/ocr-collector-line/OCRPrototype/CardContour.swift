import CoreGraphics
import Foundation

/// How the collector line is found in the image, switchable on screen to compare the two.
enum ScanMode: String, CaseIterable, Identifiable {
    /// The fixed strip at the bottom of the outline (`ScanGuide.regionOfInterest`).
    case fixedStrip
    /// The card's edges, detected on every frame: the strip follows the card, even tilted.
    case cardContour

    var id: Self { self }
}

/// A card found in the image: its four corners, normalised on the upright portrait image, origin
/// top-left (as drawn on the preview). Pure geometry.
struct CardContour: Equatable {
    let topLeft: CGPoint
    let topRight: CGPoint
    let bottomRight: CGPoint
    let bottomLeft: CGPoint

    init(topLeft: CGPoint, topRight: CGPoint, bottomRight: CGPoint, bottomLeft: CGPoint) {
        self.topLeft = topLeft
        self.topRight = topRight
        self.bottomRight = bottomRight
        self.bottomLeft = bottomLeft
    }

    /// From the corners of a Vision rectangle (origin bottom-left, on the image as oriented for
    /// the request). Nil when the rectangle lies on its side: Vision names the corners after their
    /// place in the image, so past 45° of tilt the card's bottom edge is no longer `bottomLeft`
    /// to `bottomRight`.
    init?(visionTopLeft: CGPoint, topRight: CGPoint, bottomRight: CGPoint, bottomLeft: CGPoint) {
        func flipped(_ point: CGPoint) -> CGPoint { CGPoint(x: point.x, y: 1 - point.y) }
        self.init(topLeft: flipped(visionTopLeft), topRight: flipped(topRight),
                  bottomRight: flipped(bottomRight), bottomLeft: flipped(bottomLeft))
        guard isPortrait else { return nil }
    }

    var corners: [CGPoint] { [topLeft, topRight, bottomRight, bottomLeft] }

    /// Taller than wide, in pixels (normalised x is a fraction of the narrower side).
    var isPortrait: Bool {
        func length(_ a: CGPoint, _ b: CGPoint) -> Double {
            hypot((a.x - b.x) * ScanGuide.imageAspect, a.y - b.y)
        }
        let height = length(topLeft, bottomLeft) + length(topRight, bottomRight)
        let width = length(topLeft, topRight) + length(bottomLeft, bottomRight)
        return height > width
    }

    /// The collector line strip, following the card's sides: from `ScanGuide.stripAboveBottom`
    /// above its bottom edge to `ScanGuide.marginBelowBottom` below it, in case Vision took the
    /// inner frame of a black-bordered card for its edge.
    var collectorStrip: CardContour {
        func along(_ top: CGPoint, _ bottom: CGPoint, _ fraction: Double) -> CGPoint {
            CGPoint(x: top.x + (bottom.x - top.x) * fraction, y: top.y + (bottom.y - top.y) * fraction)
        }
        let top = 1 - ScanGuide.stripAboveBottom
        let bottom = 1 + ScanGuide.marginBelowBottom
        return CardContour(topLeft: along(topLeft, bottomLeft, top), topRight: along(topRight, bottomRight, top),
                           bottomRight: along(topRight, bottomRight, bottom),
                           bottomLeft: along(topLeft, bottomLeft, bottom))
    }
}
