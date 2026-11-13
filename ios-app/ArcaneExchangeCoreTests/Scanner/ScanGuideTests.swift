import CoreGraphics
import Foundation
import Testing

@testable import ArcaneExchangeCore

struct ScanGuideTests {
    @Test func theCardOutlineKeepsTheCardProportionsInTheVideoImage() {
        let card = ScanGuide.card
        // Normalised width and height are fractions of a 9:16 portrait image.
        let pixelAspect = (card.width * 9) / (card.height * 16)
        #expect(abs(pixelAspect - 63.0 / 88.0) < 0.001)
        #expect(card.minX > 0 && card.maxX < 1 && card.minY > 0 && card.maxY < 1)
    }

    @Test func focusAimsAtThePrintedLineInSensorCoordinates() {
        let card = ScanGuide.card
        let point = ScanGuide.focusPoint
        // Back the sensor point out to portrait coordinates: sensor (x, y) is portrait (1 - y, x).
        let portrait = CGPoint(x: 1 - point.y, y: point.x)
        #expect(portrait.x > card.minX && portrait.x < card.midX)
        #expect(portrait.y > card.maxY - card.height / 10 && portrait.y < card.maxY)
    }

    @Test func zoomPushesTheCardBeyondTheMinimumFocusDistance() {
        // ~70° wide lens: the card fills the outline at about 10 cm, it focuses from 20 cm.
        let zoom = ScanGuide.zoomFactor(horizontalFieldOfViewDegrees: 70, minimumFocusDistanceMillimetres: 200)
        #expect(abs(zoom - 2.2) < 0.05)
    }

    @Test func noZoomWhenTheLensFocusesCloseEnough() {
        #expect(ScanGuide.zoomFactor(horizontalFieldOfViewDegrees: 70, minimumFocusDistanceMillimetres: 50) == 1)
    }
}
