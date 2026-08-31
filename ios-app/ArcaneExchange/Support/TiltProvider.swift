import CoreMotion
import SwiftUI

/// Pure maths for the foil effect. Kept off any `View` type so it stays testable — see the
/// note in [[ios-view-static-func-maactor-test-crash]].
enum FoilMath {
    /// Maps a raw gravity component (roughly −1…1, ~0 with the device held upright) to a
    /// tilt scalar. The gain is high so a small, comfortable tilt already swings the foil.
    static func normalizedTilt(_ raw: Double, gain: Double = 2.6) -> Double {
        min(1, max(-1, raw * gain))
    }

    /// Where a cell sits in its scroll view, 0 (top) → 1 (bottom), with a little overscan so
    /// the shimmer keeps moving as a card enters and leaves. 0.5 when there is no scroll view.
    static func scrollProgress(midY: Double, scrollHeight: Double) -> Double {
        guard scrollHeight > 0 else { return 0.5 }
        return min(1.3, max(-0.3, midY / scrollHeight))
    }

    /// Same thing recentred to −1…1, the form the gradient offsets want.
    static func sweep(midY: Double, scrollHeight: Double) -> Double {
        (scrollProgress(midY: midY, scrollHeight: scrollHeight) - 0.5) * 2
    }
}

/// One app-wide CoreMotion source for the holographic foil, published as a tilt scalar the
/// foil overlays read. Device-motion updates run only while at least one foil card is on
/// screen (`retain()` / `release()` are balanced from `FoilOverlay`), and never under Low
/// Power Mode or Reduce Motion.
@MainActor
@Observable
final class TiltProvider {
    /// Left/right tilt, −1…1, ~0 at rest.
    private(set) var roll: Double = 0
    /// Front/back tilt, −1…1, ~0 at rest, +1 laid flat.
    private(set) var pitch: Double = 0

    @ObservationIgnored private let motion = CMMotionManager()
    @ObservationIgnored private var subscribers = 0

    func retain() {
        subscribers += 1
        if subscribers == 1 {
            start()
        }
    }

    func release() {
        subscribers = max(0, subscribers - 1)
        if subscribers == 0 {
            stop()
        }
    }

    private func start() {
        guard motion.isDeviceMotionAvailable,
              !ProcessInfo.processInfo.isLowPowerModeEnabled,
              !UIAccessibility.isReduceMotionEnabled
        else { return }

        motion.deviceMotionUpdateInterval = 1.0 / 30.0
        motion.startDeviceMotionUpdates(to: .main) { [weak self] data, _ in
            guard let self, let gravity = data?.gravity else { return }
            let newRoll = FoilMath.normalizedTilt(gravity.x)
            let newPitch = FoilMath.normalizedTilt(-gravity.z)
            // Only publish real movement — every change re-evaluates the foil overlays.
            if abs(newRoll - roll) > 0.01 {
                roll = newRoll
            }
            if abs(newPitch - pitch) > 0.01 {
                pitch = newPitch
            }
        }
    }

    private func stop() {
        motion.stopDeviceMotionUpdates()
        roll = 0
        pitch = 0
    }
}
