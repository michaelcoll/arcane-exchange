#include <metal_stdlib>
#include <SwiftUI/SwiftUI_Metal.h>

using namespace metal;

/// The classic holo ramp, as six stops. Deliberately uneven in hue (roughly 2°, 53°, 93°,
/// 176°, 228°, 283°): an evenly spaced spectrum reads as a printed rainbow rather than as
/// diffracted light. Same palette the reference CSS calls "sunpillar".
constant float3 kSunpillar[6] = {
    float3(1.000, 0.478, 0.460),
    float3(1.000, 0.928, 0.380),
    float3(0.659, 1.000, 0.380),
    float3(0.520, 1.000, 0.968),
    float3(0.480, 0.584, 1.000),
    float3(0.847, 0.460, 1.000)
};

static float3 sunpillar(float turns) {
    float x = fract(turns) * 6.0;
    int i = int(x);
    return mix(kSunpillar[i], kSunpillar[(i + 1) % 6], fract(x));
}

/// Photoshop `overlay`: multiply in the shadows, screen in the highlights.
static float3 overlayBlend(float3 base, float3 blend) {
    return select(2.0 * base * blend,
                  1.0 - 2.0 * (1.0 - base) * (1.0 - blend),
                  base > 0.5);
}

/// Traditional Magic foil, applied over a card scan.
///
/// A traditional foil is a holographic laminate sitting *over* the ink, on top of a white
/// under-print plate — so the shimmer flares where the card is already light and stays quiet
/// in the shadows. Three layers, matching the structure of a real sheet:
///
/// 1. diagonal rainbow bands, whose colour depends on the viewing angle;
/// 2. the fine ruling milled into the foil stock, blended `overlay` into the bands;
/// 3. a broad highlight rolling across as the card turns.
///
/// The composite is a **colour dodge** (`base / (1 - coat)`), which is what makes it read as
/// foil rather than as a gradient laid on top: it divides by the inverse of the coat, so light
/// ink blows out into full spectrum while shadow stays shadow. The artwork's own luminance
/// does that work — no explicit mask involved.
///
/// - Parameters:
///   - size: the card's size in points, to normalise `position`.
///   - tilt: device roll/pitch, −1…1 (`TiltProvider`).
///   - slide: where the card sits in its scroll view, −1…1 (`FoilMath.sweep`).
[[ stitchable ]] half4 cardFoil(float2 position,
                                SwiftUI::Layer layer,
                                float2 size,
                                float2 tilt,
                                float slide)
{
    half4 src = layer.sample(position);
    // Rounded corners and any transparent padding take no coating.
    if (src.a < 0.004h) {
        return src;
    }

    float alpha = float(src.a);
    float3 base = float3(src.rgb) / alpha; // layer samples arrive premultiplied

    float2 uv = position / max(size, float2(1.0));

    // Where the light is coming from. Tilting the device moves it directly; so does scrolling
    // the collection past the card, weighted so a card crossing the screen rolls the bands
    // through about a full turn — alive under the thumb even when the phone is held still.
    float2 view = float2(tilt.x, tilt.y + slide * 0.9);

    // 1. Rainbow bands running across the card at roughly 110°, sliding as the angle changes.
    float across = dot(uv, float2(0.94, 0.34));
    float3 bands = sunpillar(across * 2.4 - view.x * 0.55 - view.y * 0.62);

    // 2. The ruling milled into the stock: fine lines that only ever darken the bands, never
    //    brighten them (black → 40 % grey, as the reference sheet does). Measured in points
    //    rather than UV, so the lines keep the same density whatever size the card is drawn at.
    float ruling = 0.20 + 0.20 * cos(position.x * (6.2831853 / 2.5));
    float3 sheet = overlayBlend(bands, float3(ruling));

    // 3. The broad highlight that rolls across as the card turns.
    float2 toHotspot = uv - (float2(0.5) - view * 0.42);
    float glare = exp(-dot(toHotspot, toHotspot) * 4.5);

    // Calibrated against a real scan: enough to read as foil, low enough to keep the card
    // legible. Raising the two constants is the knob if it should catch the eye harder.
    float3 coat = saturate(sheet * (0.16 + 0.34 * glare));

    float3 dodged = saturate(base / max(1.0 - coat, 1e-3));
    float3 lit = mix(base, dodged, 0.9);

    return half4(half3(lit * alpha), src.a);
}
