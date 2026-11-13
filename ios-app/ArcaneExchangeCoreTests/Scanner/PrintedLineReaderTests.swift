import Testing

@testable import ArcaneExchangeCore

struct PrintedLineReaderTests {
    @Test(arguments: PrintedLineFixtures.all)
    func readsTheFixture(_ fixture: PrintedLineFixture) {
        #expect(PrintedLineReader.read(lines: fixture.lines) == fixture.expected, "\(fixture.name)")
    }
}
