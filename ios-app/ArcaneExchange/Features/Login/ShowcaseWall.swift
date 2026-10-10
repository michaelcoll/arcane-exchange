import SwiftUI

/// The wall of cards behind the login card (`.aw-wall` in the iOS mockup): three tilted columns
/// of whole cards drifting slowly, the middle one the other way, fading into the app background
/// over the lower half.
///
/// The wall stands still under « Réduire les animations » and in Low Power Mode.
struct ShowcaseWall: View {
    /// The cards, most expensive first, loaded beforehand so the wall appears whole. Not empty.
    let images: [UIImage]

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isLowPowerModeEnabled = ProcessInfo.processInfo.isLowPowerModeEnabled

    /// How far the wall overflows the screen, so that tilting it uncovers no corner.
    private static let topOverflow = 60.0
    private static let sideOverflow = 70.0
    private static let spacing = 12.0
    private static let tilt = Angle.degrees(-9)
    /// Width over height of a card.
    private static let cardRatio = 63.0 / 88.0
    private static let columns = [
        ShowcaseDrift(duration: 46, isReversed: false, headStart: 0),
        ShowcaseDrift(duration: 46, isReversed: true, headStart: 90),
        ShowcaseDrift(duration: 58, isReversed: false, headStart: 0)
    ]

    private var isStill: Bool {
        reduceMotion || isLowPowerModeEnabled
    }

    var body: some View {
        GeometryReader { proxy in
            let width = proxy.size.width + 2 * Self.sideOverflow
            let height = proxy.size.height + Self.topOverflow
            let count = Double(Self.columns.count)
            let cardWidth = (width - (count - 1) * Self.spacing) / count
            let cardSize = CGSize(width: cardWidth, height: cardWidth / Self.cardRatio)
            // One loop of a column must cover the wall on its own, whatever its head start.
            let tallest = height + (Self.columns.map(\.headStart).max() ?? 0)
            let minimum = Int((tallest / (cardSize.height + Self.spacing)).rounded(.up))
            let imageColumns = ShowcaseWallLayout.columns(
                images,
                count: Self.columns.count,
                minimumPerColumn: minimum
            )

            HStack(alignment: .top, spacing: Self.spacing) {
                ForEach(Array(imageColumns.enumerated()), id: \.offset) { index, images in
                    ShowcaseColumn(
                        images: images,
                        cardSize: cardSize,
                        spacing: Self.spacing,
                        height: height,
                        drift: Self.columns[index],
                        isStill: isStill
                    )
                }
            }
            // A new identity restarts the columns from rest when the motion setting changes.
            .id(isStill)
            .frame(width: width, height: height, alignment: .top)
            // The wall is drawn as one layer (opacity), which cuts the columns at its frame.
            .clipped()
            .position(x: proxy.size.width / 2, y: (proxy.size.height - Self.topOverflow) / 2)
            .rotationEffect(Self.tilt)
            .opacity(0.85)
        }
        .overlay { fade }
        .clipped()
        // One layer, so that fading the wall in never shows the cards the fade hides.
        .compositingGroup()
        .allowsHitTesting(false)
        .accessibilityHidden(true)
        .task {
            let changes = NotificationCenter.default
                .notifications(named: .NSProcessInfoPowerStateDidChange)
                .map { _ in () }
            for await _ in changes {
                isLowPowerModeEnabled = ProcessInfo.processInfo.isLowPowerModeEnabled
            }
        }
    }

    /// The wall dissolves into the app background, light or dark, where the login card sits.
    private var fade: some View {
        let background = Color(.systemBackground)
        return LinearGradient(
            stops: [
                .init(color: background.opacity(0), location: 0),
                .init(color: background.opacity(0.4), location: 0.3),
                .init(color: background, location: 0.62)
            ],
            startPoint: .top,
            endPoint: .bottom
        )
    }
}

/// How one column moves: seconds per loop, direction, and how many points up the loop it
/// starts from.
private struct ShowcaseDrift {
    let duration: Double
    let isReversed: Bool
    let headStart: Double
}

/// One column of the wall: its cards twice over, drifting by exactly one copy so the loop has
/// no seam.
private struct ShowcaseColumn: View {
    let images: [UIImage]
    let cardSize: CGSize
    let spacing: Double
    let height: Double
    let drift: ShowcaseDrift
    let isStill: Bool

    @State private var hasDrifted = false

    var body: some View {
        let loop = Double(images.count) * (cardSize.height + spacing)

        VStack(spacing: spacing) {
            ForEach(0 ..< images.count * 2, id: \.self) { index in
                Image(uiImage: images[index % images.count])
                    .resizable()
                    .scaledToFill()
                    .frame(width: cardSize.width, height: cardSize.height)
                    .clipShape(RoundedRectangle(cornerRadius: cardSize.width * 0.048, style: .continuous))
                    // Per card: a shadow on the column would flatten it into one layer, too
                    // tall to be drawn at all.
                    .shadow(color: .black.opacity(0.35), radius: 10, y: 10)
            }
        }
        // Upwards from rest, or back down to rest for a reversed column.
        .offset(y: (hasDrifted != drift.isReversed ? -loop : 0) - drift.headStart)
        // The frame of the wall, not of the cards: what overflows it is cut off by the wall.
        .frame(width: cardSize.width, height: height, alignment: .top)
        .onAppear {
            guard !isStill else { return }
            withAnimation(.linear(duration: drift.duration).repeatForever(autoreverses: false)) {
                hasDrifted = true
            }
        }
    }
}
