import APIClient
import ArcaneCore
import NukeUI
import SwiftUI

/// A player's avatar: their `GET /user/{username}` image once it resolves, initials before
/// that and whenever they have none — mirrors the web client's `PlayerAvatar.vue`, neutral
/// like it: an avatar sits next to handles that already carry the color roles.
struct PlayerAvatar: View {
    let username: String
    var size: CGFloat = 36

    @Environment(\.avatarStore) private var store
    @State private var avatarURL: URL?

    var body: some View {
        Circle()
            .fill(Color(.tertiarySystemFill))
            .frame(width: size, height: size)
            .overlay { content }
            .clipShape(Circle())
            // Keyed on the username: switching player drops the previous avatar first
            // (below), so a stale image never lingers under a new handle while the new one
            // loads — same guard as the web client's `watch` on `props.username`.
            .task(id: username) {
                avatarURL = nil
                avatarURL = await store.url(for: username)
            }
    }

    @ViewBuilder private var content: some View {
        if let avatarURL {
            LazyImage(url: avatarURL) { state in
                if let image = state.image {
                    image.resizable().scaledToFill()
                } else {
                    // No spinner: the web client never shows a loading state either, only
                    // initials until the image is ready or the request fails.
                    initials
                }
            }
        } else {
            initials
        }
    }

    private var initials: some View {
        Text(PlayerMonogram.initials(from: username))
            .font(.system(size: size * 0.4, weight: .bold))
            .foregroundStyle(.secondary)
    }
}

#Preview {
    HStack {
        PlayerAvatar(username: "mizzix_42")
        PlayerAvatar(username: "golgari.jo", size: 48)
        PlayerAvatar(username: "x")
    }
    .padding()
}
