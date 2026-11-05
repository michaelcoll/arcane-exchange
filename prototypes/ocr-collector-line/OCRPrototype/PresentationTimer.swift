import Foundation

/// Measures how long Vision takes to give a stable reading of a card shown to the camera.
///
/// Method: a card is "presented" at the first frame where Vision recognises at least
/// `minLines` lines of text after the lens has been empty for `idleGap` seconds. It is
/// "recognised" at the first later frame whose set of lines equals the previous frame's.
/// The latency is the time between those two frames, so it includes the OCR time of every
/// frame in between and the one-frame stabilisation cost.
struct PresentationTimer {
    var minLines = 3
    var idleGap: TimeInterval = 1.0

    private var presentedAt: TimeInterval?
    private var lastSeenAt: TimeInterval = 0
    private var previousLines: Set<String>?
    private var previousReading: CollectorReading?
    private var reported = false

    /// Feed every frame's recognised lines. Returns the latency once per presentation.
    mutating func observe(lines: [String], at time: TimeInterval) -> TimeInterval? {
        if presentedAt != nil, time - lastSeenAt >= idleGap { reset() }
        guard lines.count >= minLines else { return nil }
        if presentedAt == nil { presentedAt = time }
        lastSeenAt = time

        let current = Set(lines)
        defer { previousLines = current }
        guard !reported, previousLines == current, let start = presentedAt else { return nil }
        reported = true
        return time - start
    }

    /// Variant for #657: the reading is retained (latency returned, once per presentation) when
    /// the collector line reader gives the same complete reading on two consecutive frames.
    /// "Presented" is unchanged. A missing or changing reading restarts the count, not the clock.
    mutating func observe(lines: [String], reading: CollectorReading?, at time: TimeInterval) -> TimeInterval? {
        if presentedAt != nil, time - lastSeenAt >= idleGap { reset() }
        guard lines.count >= minLines else { return nil }
        if presentedAt == nil { presentedAt = time }
        lastSeenAt = time

        defer { previousReading = reading }
        guard !reported, let reading, previousReading == reading, let start = presentedAt else { return nil }
        reported = true
        return time - start
    }

    private mutating func reset() {
        presentedAt = nil
        previousReading = nil
        previousLines = nil
        reported = false
    }
}
