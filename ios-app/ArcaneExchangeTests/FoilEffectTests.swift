import Testing

@testable import ArcaneExchange

struct FoilMathTests {
    @Test func normalizedTiltAppliesGainThenClamps() {
        #expect(FoilMath.normalizedTilt(0) == 0)
        #expect(abs(FoilMath.normalizedTilt(0.1, gain: 2) - 0.2) < 1e-9)
        #expect(FoilMath.normalizedTilt(2) == 1)
        #expect(FoilMath.normalizedTilt(-2) == -1)
        // The default gain is aggressive on purpose: a modest tilt already saturates.
        #expect(FoilMath.normalizedTilt(0.5) == 1)
    }

    @Test func scrollProgressIsNeutralWithoutAScrollView() {
        #expect(FoilMath.scrollProgress(midY: 400, scrollHeight: 0) == 0.5)
    }

    @Test func scrollProgressTracksPositionAndClampsWithOverscan() {
        #expect(FoilMath.scrollProgress(midY: 0, scrollHeight: 800) == 0)
        #expect(FoilMath.scrollProgress(midY: 400, scrollHeight: 800) == 0.5)
        #expect(FoilMath.scrollProgress(midY: 800, scrollHeight: 800) == 1)
        // Overscan: a cell well past the bottom still moves, but is capped.
        #expect(FoilMath.scrollProgress(midY: 4000, scrollHeight: 800) == 1.3)
        #expect(FoilMath.scrollProgress(midY: -4000, scrollHeight: 800) == -0.3)
    }

    @Test func sweepRecentresProgressToPlusMinusOne() {
        #expect(FoilMath.sweep(midY: 400, scrollHeight: 800) == 0)
        #expect(FoilMath.sweep(midY: 0, scrollHeight: 800) == -1)
        #expect(FoilMath.sweep(midY: 800, scrollHeight: 800) == 1)
    }
}
