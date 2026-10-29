import APIClient
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

/// Session cache of avatar URLs, keyed by username: one `GET /user/{username}` per player for
/// the life of the process, shared by every `PlayerAvatar` on screen — the collection grid,
/// search results, and a trade's two rails can all name the same player. Mirrors the web
/// client's module-level cache in `useUserAvatar`.
actor PlayerAvatarStore {
    private let api: any APIProtocol

    /// `.some(url)` — has an avatar (`url` may itself be `nil`, meaning confirmed none).
    /// Absent — never resolved, or the last attempt failed transiently and is owed a retry.
    private var cached: [String: URL?] = [:]
    private var inFlight: [String: Task<FetchOutcome, Never>] = [:]

    init(api: any APIProtocol) {
        self.api = api
    }

    func url(for username: String) async -> URL? {
        if let cached = cached[username] {
            return cached
        }

        let outcome: FetchOutcome
        if let task = inFlight[username] {
            outcome = await task.value
        } else {
            let task = Task<FetchOutcome, Never> { [api] in await Self.fetch(username: username, api: api) }
            inFlight[username] = task
            outcome = await task.value
            inFlight[username] = nil
        }

        switch outcome {
        case let .definitive(url):
            cached[username] = url
            return url
        case .retryLater:
            return nil
        }
    }

    private enum FetchOutcome {
        case definitive(URL?)
        case retryLater
    }

    /// A `user_not_found` (unknown username) is cached as "no avatar", same as the web client — from an
    /// avatar's point of view the two look identical. Any other failure (network, 5xx, an
    /// expired token) must not freeze the player on initials for the rest of the session, so
    /// it is left uncached for the next `PlayerAvatar` to retry. Nothing is shown for a failure:
    /// initials are a fine answer, an error message under a handle is not.
    private static func fetch(username: String, api: any APIProtocol) async -> FetchOutcome {
        do {
            let profile = try await api.get_user_profile(path: .init(username: username)).ok.body.json
            return .definitive(profile.avatar_url.flatMap(URL.init(string:)))
        } catch {
            return APIError.read(error).code == "user_not_found" ? .definitive(nil) : .retryLater
        }
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
