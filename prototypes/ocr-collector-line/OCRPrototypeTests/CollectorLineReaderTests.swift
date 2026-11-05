import Testing
@testable import OCRPrototype

struct CollectorLineReaderTests {
    @Test(arguments: CollectorLineFixtures.all)
    func readsTheFixture(_ fixture: CollectorLineFixture) {
        #expect(CollectorLineReader.read(lines: fixture.lines) == fixture.expected, "\(fixture.name)")
    }
}

struct ReadingLatencyTests {
    let reading = CollectorReading(set: "mom", number: "123", language: "en", isToken: false)
    let other = CollectorReading(set: "mom", number: "124", language: "en", isToken: false)
    let lines = ["Elesh Norn", "R 0123", "MOM • EN"]

    @Test func retainsAReadingSeenOnTwoConsecutiveFramesAndReportsTheLatency() {
        var timer = PresentationTimer()
        #expect(timer.observe(lines: lines, reading: reading, at: 1.0) == nil)
        #expect(timer.observe(lines: lines, reading: reading, at: 1.5) == 0.5)
    }

    @Test func aDifferentReadingRestartsTheCountButNotTheClock() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 1.0)
        #expect(timer.observe(lines: lines, reading: other, at: 1.2) == nil)
        #expect(timer.observe(lines: lines, reading: other, at: 1.5) == 0.5)
    }

    @Test func aFrameWithoutReadingBreaksTheSequence() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 1.0)
        _ = timer.observe(lines: lines, reading: nil, at: 1.2)
        #expect(timer.observe(lines: lines, reading: reading, at: 1.4) == nil)
    }

    @Test func anIncompleteReadingIsNeverRetained() {
        var timer = PresentationTimer()
        #expect(timer.observe(lines: lines, reading: nil, at: 1.0) == nil)
        #expect(timer.observe(lines: lines, reading: nil, at: 1.5) == nil)
    }
}
