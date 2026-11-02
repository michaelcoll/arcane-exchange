import Foundation
import Testing

@testable import ArcaneExchange

struct DeviceMemoryTests {
    private struct Unencodable: Encodable {
        func encode(to _: any Encoder) throws {
            throw EncodingError.invalidValue(0, .init(codingPath: [], debugDescription: "no"))
        }
    }

    @Test func readsBackWhatWasWritten() {
        let memory = DeviceMemory(defaults: isolatedDefaults())
        memory.write(["Sol Ring"], for: PlayerKey.searchRecentCardQueries)

        #expect(memory.read([String].self, for: PlayerKey.searchRecentCardQueries) == ["Sol Ring"])
    }

    @Test func readsNothingWhenAbsent() {
        let memory = DeviceMemory(defaults: isolatedDefaults())

        #expect(memory.read([String].self, for: PlayerKey.searchRecentCardQueries) == nil)
    }

    @Test func readsNothingWhenUnreadable() {
        let defaults = isolatedDefaults()
        defaults.set(Data("not json".utf8), forKey: "search.recentCardQueries")

        #expect(DeviceMemory(defaults: defaults).read([String].self, for: PlayerKey.searchRecentCardQueries) == nil)
    }

    @Test func removesAValue() {
        let memory = DeviceMemory(defaults: isolatedDefaults())
        memory.write(["Sol Ring"], for: PlayerKey.searchRecentCardQueries)
        memory.remove(PlayerKey.searchRecentCardQueries)

        #expect(memory.read([String].self, for: PlayerKey.searchRecentCardQueries) == nil)
    }

    @Test func keepsThePreviousValueWhenEncodingFails() {
        let memory = DeviceMemory(defaults: isolatedDefaults())
        memory.write(["Sol Ring"], for: PlayerKey.searchRecentCardQueries)
        memory.write(Unencodable(), for: PlayerKey.searchRecentCardQueries)

        #expect(memory.read([String].self, for: PlayerKey.searchRecentCardQueries) == ["Sol Ring"])
    }

    @Test func forgettingThePlayerScopeSparesForeignKeys() {
        let defaults = isolatedDefaults()
        let memory = DeviceMemory(defaults: defaults)
        for key in PlayerKey.allCases { memory.write(["x"], for: key) }
        defaults.set("https://example.test", forKey: "api_base_url")
        defaults.set("kept", forKey: "foreign.library.key")
        memory.forgetPlayerScope()

        for key in PlayerKey.allCases { #expect(memory.read([String].self, for: key) == nil) }
        #expect(defaults.string(forKey: "api_base_url") == "https://example.test")
        #expect(defaults.string(forKey: "foreign.library.key") == "kept")
    }

    /// Renaming a key would strand the saved values of signed-in players and leave orphans that
    /// sign-out never erases.
    @Test func pinsTheStoredKeyNames() {
        #expect(PlayerKey.allCases.map(\.rawValue).sorted() == ["search.recentCardQueries", "search.recentPlayers"])
    }
}
