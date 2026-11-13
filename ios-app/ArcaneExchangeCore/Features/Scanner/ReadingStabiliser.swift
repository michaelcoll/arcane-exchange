import Foundation

/// Keeps a recognised card only once it was seen on two consecutive frames: a single misread frame
/// never reaches the scanner's result.
///
/// Cards can be swapped in front of the lens without turning it away: a stable card different from
/// the retained one is a new card. The same card shown again is only retained again after the lens
/// has had no text for `idleGap` seconds.
package struct ReadingStabiliser {
    package var idleGap: TimeInterval = 1.0

    private var isPresenting = false
    private var lastSeenAt: TimeInterval = 0
    private var previous: RecognisedCard?
    /// The last card retained, until the lens is empty for `idleGap`.
    private var retained: RecognisedCard?

    package init() {}

    /// Feed every frame: the card recognised in it (nil when none) and whether Vision read any text.
    /// Returns the card when it is retained, which happens once per presentation.
    package mutating func observe(_ card: RecognisedCard?, hasText: Bool, at time: TimeInterval) -> RecognisedCard? {
        if isPresenting || retained != nil, time - lastSeenAt >= idleGap {
            reset()
        }
        guard hasText else { return nil }
        lastSeenAt = time

        defer { previous = card }
        if let retained, card == retained {
            // The retained card is still there: whatever came between was a blurred frame of it.
            isPresenting = false
            return nil
        }
        isPresenting = true
        guard let card, previous == card else { return nil }
        retained = card
        isPresenting = false
        return card
    }

    private mutating func reset() {
        isPresenting = false
        previous = nil
        retained = nil
    }
}
