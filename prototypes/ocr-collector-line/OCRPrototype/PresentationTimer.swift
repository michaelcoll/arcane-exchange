import Foundation

/// Measures how long Vision takes to give a stable reading of a card shown to the camera.
///
/// Method: a card is "presented" at the first frame where Vision recognises at least
/// `minLines` lines of text and the collector line no longer reads as the card retained before
/// (another reading, or none), or after the lens has been empty for `idleGap` seconds. It is
/// "recognised" at the first later frame whose reading equals the previous frame's. The latency
/// is the time between those two frames, so it includes the OCR time of every frame in between
/// and the one-frame stabilisation cost.
///
/// Cards can be swapped in front of the lens without turning it away: a stable reading different
/// from the retained one is a new card. The same card shown again is only retained again after
/// the lens has been empty for `idleGap`.
struct PresentationTimer {
    var minLines = 3
    var idleGap: TimeInterval = 1.0

    private var presentedAt: TimeInterval?
    private var lastSeenAt: TimeInterval = 0
    private var previousReading: CollectorReading?
    /// The last reading reported, until the lens is empty for `idleGap`.
    private var retained: CollectorReading?

    /// Feed every frame's recognised lines and reading. A reading is retained (latency returned)
    /// when the collector line reader gives the same complete reading on two consecutive frames
    /// and it differs from the reading retained before.
    /// A missing or changing reading restarts the count, not the clock.
    mutating func observe(lines: [String], reading: CollectorReading?, at time: TimeInterval) -> TimeInterval? {
        if presentedAt != nil || retained != nil, time - lastSeenAt >= idleGap { reset() }
        guard lines.count >= minLines else { return nil }
        lastSeenAt = time

        defer { previousReading = reading }
        if let retained, reading == retained {
            // The retained card is still there: whatever came between was a blurred frame of it.
            presentedAt = nil
            return nil
        }
        if presentedAt == nil { presentedAt = time }
        guard let reading, previousReading == reading, let start = presentedAt else { return nil }
        retained = reading
        presentedAt = nil
        return time - start
    }

    private mutating func reset() {
        presentedAt = nil
        previousReading = nil
        retained = nil
    }
}
