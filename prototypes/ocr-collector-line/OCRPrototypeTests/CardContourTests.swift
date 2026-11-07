import CoreGraphics
import Foundation
import Testing
@testable import OCRPrototype

struct CardContourTests {
    /// The outline of `ScanGuide`, as Vision gives it (origin bottom-left).
    let upright = ScanGuide.card

    private func fromVision(_ rect: CGRect) -> CardContour? {
        CardContour(visionTopLeft: CGPoint(x: rect.minX, y: 1 - rect.minY),
                    topRight: CGPoint(x: rect.maxX, y: 1 - rect.minY),
                    bottomRight: CGPoint(x: rect.maxX, y: 1 - rect.maxY),
                    bottomLeft: CGPoint(x: rect.minX, y: 1 - rect.maxY))
    }

    @Test func visionCornersAreFlippedToTheTopLeftOrigin() throws {
        let contour = try #require(fromVision(upright))
        #expect(abs(contour.topLeft.y - upright.minY) < 1e-9 && contour.topLeft.x == upright.minX)
        #expect(abs(contour.bottomRight.y - upright.maxY) < 1e-9 && contour.bottomRight.x == upright.maxX)
    }

    @Test func aRectangleLyingOnItsSideIsNoCard() {
        // A card's height across the image: 88 mm wide, 63 mm high, in a 9:16 image.
        let width = 0.8
        let height = width * ScanGuide.imageAspect * 63 / 88
        #expect(fromVision(CGRect(x: 0.1, y: 0.3, width: width, height: height)) == nil)
    }

    @Test func aTiltedCardIsACard() {
        // The outline turned by 20° around its centre, in pixels (x scaled by the image aspect).
        let angle = 20.0 * .pi / 180
        let centre = CGPoint(x: upright.midX * ScanGuide.imageAspect, y: upright.midY)
        func turned(_ x: Double, _ y: Double) -> CGPoint {
            let dx = x * ScanGuide.imageAspect - centre.x, dy = y - centre.y
            let pixel = CGPoint(x: centre.x + dx * cos(angle) - dy * sin(angle),
                                y: centre.y + dx * sin(angle) + dy * cos(angle))
            return CGPoint(x: pixel.x / ScanGuide.imageAspect, y: 1 - pixel.y)
        }
        let contour = CardContour(visionTopLeft: turned(upright.minX, upright.minY),
                                  topRight: turned(upright.maxX, upright.minY),
                                  bottomRight: turned(upright.maxX, upright.maxY),
                                  bottomLeft: turned(upright.minX, upright.maxY))
        #expect(contour != nil)
    }

    @Test func theCollectorStripIsTheBottomOfTheCardAndABitBelow() throws {
        let strip = try #require(fromVision(upright)).collectorStrip
        let top = upright.maxY - upright.height * ScanGuide.stripAboveBottom
        let bottom = upright.maxY + upright.height * ScanGuide.marginBelowBottom
        #expect(abs(strip.topLeft.y - top) < 1e-9 && abs(strip.topRight.y - top) < 1e-9)
        #expect(abs(strip.bottomLeft.y - bottom) < 1e-9 && abs(strip.bottomRight.y - bottom) < 1e-9)
        #expect(strip.topLeft.x == upright.minX && strip.bottomRight.x == upright.maxX)
    }

    @Test func theCollectorStripFollowsTheCardsSides() {
        // A card leaning right: its bottom edge is shifted right of its top edge.
        let contour = CardContour(topLeft: CGPoint(x: 0.1, y: 0.1), topRight: CGPoint(x: 0.8, y: 0.1),
                                  bottomRight: CGPoint(x: 0.9, y: 0.6), bottomLeft: CGPoint(x: 0.2, y: 0.6))
        let strip = contour.collectorStrip
        let shift = 0.1 * (1 + ScanGuide.marginBelowBottom)
        #expect(abs(strip.bottomLeft.x - (0.1 + shift)) < 1e-9)
        #expect(strip.topLeft.x > 0.1 && strip.topLeft.x < 0.2)
    }
}
