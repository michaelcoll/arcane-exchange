import Testing
@testable import OCRPrototype

struct CollectorLineReaderTests {
    @Test(arguments: CollectorLineFixtures.all)
    func readsTheFixture(_ fixture: CollectorLineFixture) {
        #expect(CollectorLineReader.read(lines: fixture.lines) == fixture.expected, "\(fixture.name)")
    }
}
