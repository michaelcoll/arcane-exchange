import SwiftUI

/// Holographic foil for a card scan, rendered by the `cardFoil` shader in `Foil.metal`.
///
/// A layer effect rather than an overlay: the shader samples the artwork and colour-dodges
/// against it, so the shimmer flares where the card is already light and stays quiet in the
/// shadows — the way a laminate over a white under-print plate actually behaves, and something
/// a stack of `LinearGradient`s cannot express.
///
/// Two inputs set the viewing angle, and the web version can only ever have one of them:
/// - **tilt** — device roll/pitch from the shared `TiltProvider`;
/// - **scroll** — where the card sits in its `ScrollView`, read through `.visualEffect`, which
///   costs nothing at layout time.
///
/// Falls back to a static gradient sheen under Reduce Motion.
struct FoilEffect: ViewModifier {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(TiltProvider.self) private var tilt: TiltProvider?

    func body(content: Content) -> some View {
        if reduceMotion {
            content.overlay { staticSheen }
        } else {
            shaded(content)
        }
    }

    private func shaded(_ content: Content) -> some View {
        // Read tilt out here so the body re-runs on motion updates and the closure below
        // captures fresh values; scroll stays the `.visualEffect` proxy's job.
        let roll = Float(tilt?.roll ?? 0)
        let pitch = Float(tilt?.pitch ?? 0)

        return content
            .visualEffect { view, proxy in
                let slide = FoilMath.sweep(
                    midY: Double(proxy.frame(in: .scrollView).midY),
                    scrollHeight: Double(proxy.bounds(of: .scrollView)?.height ?? 0)
                )
                return view.layerEffect(
                    ShaderLibrary.cardFoil(
                        .float2(proxy.size),
                        .float2(roll, pitch),
                        .float(Float(slide))
                    ),
                    // The shader only ever reads the pixel it is writing.
                    maxSampleOffset: .zero
                )
            }
            .onAppear { tilt?.retain() }
            .onDisappear { tilt?.release() }
    }

    private var staticSheen: some View {
        LinearGradient(
            colors: [.cyan.opacity(0.2), .clear, .purple.opacity(0.16), .clear],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
        .blendMode(.plusLighter)
        .allowsHitTesting(false)
    }
}

extension View {
    /// Laminates a card scan with the holographic foil. A no-op for a non-foil card.
    @ViewBuilder
    func foil(_ isFoil: Bool) -> some View {
        if isFoil {
            modifier(FoilEffect())
        } else {
            self
        }
    }
}
