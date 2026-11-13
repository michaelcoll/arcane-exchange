import Testing

@testable import ArcaneExchangeCore

struct ReadingStabiliserTests {
    let card = RecognisedCard(set: "MOM", number: "123", language: .english, rarity: "R", name: "Elesh Norn",
                              frenchName: nil)
    let other = RecognisedCard(set: "MOM", number: "124", language: .english, rarity: "R", name: "Other",
                               frenchName: nil)

    @Test func retainsACardSeenOnTwoConsecutiveFrames() {
        var stabiliser = ReadingStabiliser()
        #expect(stabiliser.observe(card, hasText: true, at: 1.0) == nil)
        #expect(stabiliser.observe(card, hasText: true, at: 1.5) == card)
    }

    @Test func retainsACardOncePerPresentation() {
        var stabiliser = ReadingStabiliser()
        _ = stabiliser.observe(card, hasText: true, at: 0)
        _ = stabiliser.observe(card, hasText: true, at: 0.2)
        #expect(stabiliser.observe(card, hasText: true, at: 0.4) == nil)
    }

    @Test func aDifferentCardRestartsTheCount() {
        var stabiliser = ReadingStabiliser()
        _ = stabiliser.observe(card, hasText: true, at: 1.0)
        #expect(stabiliser.observe(other, hasText: true, at: 1.2) == nil)
        #expect(stabiliser.observe(other, hasText: true, at: 1.5) == other)
    }

    @Test func aFrameWithoutCardBreaksTheSequence() {
        var stabiliser = ReadingStabiliser()
        _ = stabiliser.observe(card, hasText: true, at: 1.0)
        _ = stabiliser.observe(nil, hasText: true, at: 1.2)
        #expect(stabiliser.observe(card, hasText: true, at: 1.4) == nil)
    }

    @Test func aFrameWithoutCardIsNeverRetained() {
        var stabiliser = ReadingStabiliser()
        #expect(stabiliser.observe(nil, hasText: true, at: 1.0) == nil)
        #expect(stabiliser.observe(nil, hasText: true, at: 1.5) == nil)
    }

    @Test func theSameCardShownAgainAfterAnEmptyLensIsRetainedAgain() {
        var stabiliser = ReadingStabiliser()
        _ = stabiliser.observe(card, hasText: true, at: 0)
        _ = stabiliser.observe(card, hasText: true, at: 0.2)
        _ = stabiliser.observe(nil, hasText: false, at: 0.5)
        #expect(stabiliser.observe(card, hasText: true, at: 5.0) == nil)
        #expect(stabiliser.observe(card, hasText: true, at: 5.25) == card)
    }

    @Test func aNewCardSwappedInWithoutEmptyingTheLensIsRetained() {
        var stabiliser = ReadingStabiliser()
        _ = stabiliser.observe(card, hasText: true, at: 0)
        _ = stabiliser.observe(card, hasText: true, at: 0.2)
        _ = stabiliser.observe(nil, hasText: true, at: 0.4)
        _ = stabiliser.observe(other, hasText: true, at: 0.6)
        #expect(stabiliser.observe(other, hasText: true, at: 0.8) == other)
    }

    @Test func theRetainedCardBlurredForAFrameIsNotRetainedAgain() {
        var stabiliser = ReadingStabiliser()
        _ = stabiliser.observe(card, hasText: true, at: 0)
        _ = stabiliser.observe(card, hasText: true, at: 0.2)
        _ = stabiliser.observe(nil, hasText: true, at: 0.4)
        #expect(stabiliser.observe(card, hasText: true, at: 0.6) == nil)
        #expect(stabiliser.observe(card, hasText: true, at: 0.8) == nil)
    }

    @Test func aFrameWithoutTextIsNoCard() {
        var stabiliser = ReadingStabiliser()
        #expect(stabiliser.observe(card, hasText: false, at: 0) == nil)
        #expect(stabiliser.observe(card, hasText: false, at: 0.2) == nil)
    }
}
