import CoreGraphics
import Foundation

/// Where the card must sit in the image, and what follows from it: the outline drawn on screen, the
/// point the lens focuses on, and the zoom that keeps the card beyond the minimum focus distance.
/// Pure geometry. Unless stated, coordinates are normalised on the upright portrait image, origin
/// top-left, and the preview shows the whole 16:9 image (`resizeAspect`) so the outline drawn on
/// screen and the image share these fractions.
package enum ScanGuide {
    package static let cardWidthMillimetres = 63.0
    /// Card width / height.
    package static let cardAspect = 63.0 / 88.0
    /// Portrait width / height of the 16:9 video image.
    package static let imageAspect = 9.0 / 16.0
    /// Share of the image width the card outline takes.
    package static let widthFraction = 0.8
    /// Gap above the outline: high in the image, so the card stays clear of the drawer at the
    /// bottom of the screen.
    package static let topFraction = 0.08
    /// The printed line sits in the bottom ~8 % of the card; read a bit more, and a bit below
    /// the card's edge, since the card is never held exactly in the outline.
    package static let stripAboveBottom = 0.16
    package static let marginBelowBottom = 0.04
    /// Hold the card a little beyond the minimum focus distance, not right on it.
    package static let focusMargin = 1.1

    /// Where the preview shows the whole image in an area of the given size: as large as fits,
    /// centred (`resizeAspect`).
    package static func fittedImage(in size: CGSize) -> CGRect {
        let width = min(size.width, size.height * imageAspect)
        let height = width / imageAspect
        return CGRect(x: (size.width - width) / 2, y: (size.height - height) / 2, width: width, height: height)
    }

    /// The card outline.
    package static var card: CGRect {
        let height = widthFraction * imageAspect / cardAspect
        return CGRect(x: (1 - widthFraction) / 2, y: topFraction, width: widthFraction, height: height)
    }

    /// Focus and exposure point on the printed line, in capture device coordinates: landscape
    /// sensor, (0, 0) top-left and (1, 1) bottom-right with the phone held in portrait, so the
    /// portrait point (x, y) is the sensor point (y, 1 - x).
    package static var focusPoint: CGPoint {
        let card = card
        let portrait = CGPoint(x: card.minX + card.width * 0.25, y: card.maxY - card.height * 0.05)
        return CGPoint(x: portrait.y, y: 1 - portrait.x)
    }

    /// The zoom at which a card filling the outline is beyond the lens's minimum focus distance
    /// (the approach of Apple's AVCamBarcode sample); 1 when the lens already focuses that close.
    /// The field of view is the format's `videoFieldOfView`, along the long side of the image.
    package static func zoomFactor(
        horizontalFieldOfViewDegrees: Double,
        minimumFocusDistanceMillimetres: Double
    ) -> Double {
        let tanHalfShortSide = tan(horizontalFieldOfViewDegrees * .pi / 360) * imageAspect
        let distanceFillingOutline = (cardWidthMillimetres / widthFraction) / (2 * tanHalfShortSide)
        return max(1, minimumFocusDistanceMillimetres * focusMargin / distanceFillingOutline)
    }
}
