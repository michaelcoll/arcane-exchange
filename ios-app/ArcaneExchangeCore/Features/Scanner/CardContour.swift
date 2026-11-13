import CoreGraphics
import Foundation

/// A card found in the image: its four corners, normalised on the upright portrait image, origin
/// top-left (as drawn on the preview). Pure geometry.
package struct CardContour: Equatable, Sendable {
    package let topLeft: CGPoint
    package let topRight: CGPoint
    package let bottomRight: CGPoint
    package let bottomLeft: CGPoint

    package init(topLeft: CGPoint, topRight: CGPoint, bottomRight: CGPoint, bottomLeft: CGPoint) {
        self.topLeft = topLeft
        self.topRight = topRight
        self.bottomRight = bottomRight
        self.bottomLeft = bottomLeft
    }

    /// From the corners of a Vision rectangle (origin bottom-left, on the image as oriented for
    /// the request). Nil when the rectangle lies on its side: Vision names the corners after their
    /// place in the image, so past 45° of tilt the card's bottom edge is no longer `bottomLeft`
    /// to `bottomRight`.
    package init?(visionTopLeft: CGPoint, topRight: CGPoint, bottomRight: CGPoint, bottomLeft: CGPoint) {
        func flipped(_ point: CGPoint) -> CGPoint {
            CGPoint(x: point.x, y: 1 - point.y)
        }
        self.init(topLeft: flipped(visionTopLeft), topRight: flipped(topRight),
                  bottomRight: flipped(bottomRight), bottomLeft: flipped(bottomLeft))
        guard isPortrait else { return nil }
    }

    /// Taller than wide, in pixels (normalised x is a fraction of the narrower side).
    package var isPortrait: Bool {
        func length(_ start: CGPoint, _ end: CGPoint) -> Double {
            hypot((start.x - end.x) * ScanGuide.imageAspect, start.y - end.y)
        }
        let height = length(topLeft, bottomLeft) + length(topRight, bottomRight)
        let width = length(topLeft, topRight) + length(bottomLeft, bottomRight)
        return height > width
    }

    /// The printed line strip, following the card's sides: from `ScanGuide.stripAboveBottom`
    /// above its bottom edge to `ScanGuide.marginBelowBottom` below it, in case Vision took the
    /// inner frame of a black-bordered card for its edge.
    package var collectorStrip: CardContour {
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
