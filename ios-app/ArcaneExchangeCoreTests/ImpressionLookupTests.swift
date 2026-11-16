import APIClient
import Testing

@testable import ArcaneExchangeCore

private func page(_ items: [SearchCard]) -> Operations.search_cards.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: Int64(items.count)))))
}

private func impression(
    _ id: String,
    name: String = "Sol Ring",
    frenchName: String? = nil,
    set: String = "FDN",
    number: String = "001"
) -> SearchCard {
    var card = searchCard(id)
    card.name = name
    card.french_name = frenchName
    card.set_code = set
    card.collector_number = number
    return card
}

struct ImpressionLookupTests {
    private func makeLookup(_ api: FakeAPI, isSignedIn: Bool = true) -> ImpressionLookup {
        ImpressionLookup(api: api, isSignedIn: { isSignedIn })
    }

    @Test func findsTheImpressionsOfATextAcrossTheWholeCatalogue() async throws {
        let api = FakeAPI()
        nonisolated(unsafe) var query: Operations.search_cards.Input.Query?
        api.onSearchCards = { input in
            query = input.query
            return page([impression("a"), impression("b", set: "C21", number: "263")])
        }

        let cards = try await makeLookup(api).impressions(matching: "Sol Ring")

        #expect(cards.map(\.scryfall_id) == ["a", "b"])
        #expect(query?.q == "Sol Ring")
        #expect(query?.owned == nil)
        #expect(query?.offered == nil)
        #expect(query?.sets == nil)
    }

    @Test func findsAnImpressionByItsSetAndNumber() async throws {
        let api = FakeAPI()
        nonisolated(unsafe) var query: Operations.search_cards.Input.Query?
        api.onSearchCards = { input in
            query = input.query
            return page([impression("a", set: "SLD", number: "123a")])
        }

        let card = try await makeLookup(api).impression(for: ImpressionKey(set: "SLD", number: "123a"))

        #expect(card?.scryfall_id == "a")
        #expect(query?.sets == "SLD")
        #expect(query?.collector_number == "123a")
        #expect(query?.q == nil)
    }

    @Test func ignoresAnImpressionItCannotFind() async throws {
        let api = FakeAPI()
        api.onSearchCards = { input in
            input.query.collector_number == "001" ? page([impression("a")]) : page([])
        }
        let keys = [ImpressionKey(set: "FDN", number: "999"), ImpressionKey(set: "FDN", number: "001")]

        let entities = try await makeLookup(api).entities(for: keys)

        #expect(entities.map(\.key) == [keys[1]])
        #expect(entities.first?.card?.scryfall_id == "a")
        #expect(try await makeLookup(api).impression(for: keys[0]) == nil)
    }

    @Test func handsTheKeysThroughWithoutCallingTheAPIWhenSignedOut() async throws {
        let api = FakeAPI()
        let keys = [ImpressionKey(set: "FDN", number: "001"), ImpressionKey(set: "SLD", number: "123a")]

        let entities = try await makeLookup(api, isSignedIn: false).entities(for: keys)

        #expect(entities.map(\.key) == keys)
        #expect(entities.allSatisfy { $0.card == nil })
        #expect(api.calls.isEmpty)
    }

    @Test func answersWithTheFreshImpression() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([impression("a")]) }

        let outcome = await makeLookup(api).card(for: ImpressionKey(set: "FDN", number: "001"))

        #expect(outcome.map(\.scryfall_id) == .success("a"))
    }

    @Test func answersSignedOutWithoutCallingTheAPI() async {
        let api = FakeAPI()

        let outcome = await makeLookup(api, isSignedIn: false).card(for: ImpressionKey(set: "FDN", number: "001"))

        #expect(outcome.map(\.scryfall_id) == .failure(.signedOut))
        #expect(api.calls.isEmpty)
    }

    @Test func answersAnImpressionItCannotFindAsNotFound() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([]) }

        let outcome = await makeLookup(api).card(for: ImpressionKey(set: "FDN", number: "999"))

        #expect(outcome.map(\.scryfall_id) == .failure(.notFound))
    }

    @Test func answersAFailedCallWithItsMessage() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "internal", status: 500) }

        let outcome = await makeLookup(api).card(for: ImpressionKey(set: "FDN", number: "001"))

        #expect(outcome.map(\.scryfall_id) == .failure(.unavailable("Erreur technique, réessaie plus tard.")))
    }

    @Test func doesNotCallTheAPIWhenSignedOut() async {
        let api = FakeAPI()

        await #expect(throws: IntentFailure.signedOut) {
            try await makeLookup(api, isSignedIn: false).impressions(matching: "Sol Ring")
        }
        #expect(api.calls.isEmpty)
    }

    @Test func readsAnExpiredSessionAsSignedOut() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "unauthorized", status: 401) }

        await #expect(throws: IntentFailure.signedOut) {
            try await makeLookup(api).impressions(matching: "Sol Ring")
        }
    }

    @Test func saysSimplyThatTheServerFailed() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "internal", status: 500) }

        await #expect(throws: IntentFailure.unavailable("Erreur technique, réessaie plus tard.")) {
            try await makeLookup(api).impressions(matching: "Sol Ring")
        }
    }
}

struct ImpressionKeyTests {
    @Test(arguments: [
        ImpressionKey(set: "FDN", number: "001"),
        ImpressionKey(set: "SLD", number: "123a"),
        ImpressionKey(set: "30A", number: "12/★"),
    ])
    func survivesARoundTripThroughItsIdentifier(key: ImpressionKey) {
        #expect(ImpressionKey(id: key.id) == key)
    }

    @Test func readsTheKeyOfAnImpression() {
        #expect(ImpressionKey(impression("a", set: "C21", number: "263")) == ImpressionKey(set: "C21", number: "263"))
    }

    @Test func ignoresAnIdentifierWithoutANumber() {
        #expect(ImpressionKey(id: "FDN") == nil)
    }
}

struct IntentFailureTests {
    private func outcome(_ result: Result<Int, IntentFailure>) -> String {
        result.text { "\($0) joueurs" }
    }

    @Test func saysTheSuccessWhenThereIsOne() {
        #expect(outcome(.success(3)) == "3 joueurs")
    }

    @Test func asksToSignInWhenSignedOut() {
        #expect(outcome(.failure(.signedOut)) == "Connecte-toi dans Arcane Exchange.")
    }

    @Test func saysTheImpressionIsUnknown() {
        #expect(outcome(.failure(.notFound)) == "Je ne trouve pas cette carte.")
    }

    @Test func relaysTheErrorMessage() {
        #expect(outcome(.failure(.unavailable("Service indisponible."))) == "Service indisponible.")
    }
}

struct ImpressionCopyTests {
    @Test func namesAnImpressionBySetAndNumber() {
        #expect(ImpressionKey(set: "FDN", number: "001").label == "FDN · 001")
    }

    @Test func tellsAnImpressionApartInEnglishWhenItHasAFrenchName() {
        let card = impression("a", name: "Sol Ring", frenchName: "Anneau solaire")

        #expect(ImpressionCopy.subtitle(of: card) == "Sol Ring · FDN · 001")
    }

    @Test func givesOnlyTheSetAndNumberInEnglishOtherwise() {
        let card = impression("a", name: "Sol Ring")

        #expect(ImpressionCopy.subtitle(of: card) == "FDN · 001")
    }

    @Test func dropsTheEnglishNameWhenItIsTheFrenchOne() {
        let card = impression("a", name: "Lightning Bolt", frenchName: "Lightning Bolt")

        #expect(ImpressionCopy.subtitle(of: card) == "FDN · 001")
    }
}
