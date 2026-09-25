import NukeUI
import SwiftUI

/// A card image, filling its frame, through `ArtworkPipeline`.
///
/// A card with no image yet (pending) or whose image fails to load shows the generic card back
/// bundled in the asset catalog, never a broken image — and never a network call for it.
///
/// `LazyImage` rather than `AsyncImage`: grids re-create cells as they scroll, and `AsyncImage`
/// has no decoded-image cache, so every reappearance meant a fresh decode and a placeholder
/// flash. Nuke serves those from memory.
struct CardImageView: View {
    let url: URL?
    var foil = false

    var body: some View {
        if let url {
            LazyImage(url: url) { state in
                if let image = state.image {
                    image.resizable().scaledToFill().foil(foil)
                } else if state.error != nil {
                    cardBack
                } else {
                    Rectangle().fill(.quaternary)
                }
            }
        } else {
            cardBack
        }
    }

    private var cardBack: some View {
        Image("CardBack")
            .resizable()
            .scaledToFill()
            .accessibilityHidden(true)
    }
}

#Preview {
    HStack {
        CardImageView(url: nil)
        CardImageView(url: URL(string: "https://example.invalid/missing.webp"))
    }
    .frame(height: 200)
    .padding()
}
