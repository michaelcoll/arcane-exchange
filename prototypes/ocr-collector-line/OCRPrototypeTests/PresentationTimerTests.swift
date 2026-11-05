import Testing
@testable import OCRPrototype

struct PresentationTimerTests {
    let card = ["Lightning Bolt", "Instant", "R", "0123 M21 EN"]

    @Test func reportsTheLatencyWhenTheReadingStabilises() {
        var timer = PresentationTimer()
        #expect(timer.observe(lines: [], at: 0) == nil)
        #expect(timer.observe(lines: card, at: 1.0) == nil)
        #expect(timer.observe(lines: card, at: 1.5) == 0.5)
    }

    @Test func reportsOnlyOncePerPresentation() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: card, at: 0)
        _ = timer.observe(lines: card, at: 0.2)
        #expect(timer.observe(lines: card, at: 0.4) == nil)
    }

    @Test func aChangingReadingIsNotStableYet() {
        var timer = PresentationTimer()
        let noisy = Array(card.dropLast()) + ["0I23 M21 EN"]
        _ = timer.observe(lines: card, at: 0)
        #expect(timer.observe(lines: noisy, at: 0.2) == nil)
        #expect(timer.observe(lines: noisy, at: 0.4) == 0.4)
    }

    @Test func aNewCardAfterAnEmptyLensStartsANewMeasure() {
        var timer = PresentationTimer()
        let other = ["Giant Growth", "Instant", "C", "0001 LEA EN"]
        _ = timer.observe(lines: card, at: 0)
        _ = timer.observe(lines: card, at: 0.2)
        _ = timer.observe(lines: [], at: 0.5)
        #expect(timer.observe(lines: other, at: 5.0) == nil)
        #expect(timer.observe(lines: other, at: 5.25) == 0.25)
    }

    @Test func tooFewLinesIsNoCard() {
        var timer = PresentationTimer()
        #expect(timer.observe(lines: ["a"], at: 0) == nil)
        #expect(timer.observe(lines: ["a"], at: 0.2) == nil)
    }
}
