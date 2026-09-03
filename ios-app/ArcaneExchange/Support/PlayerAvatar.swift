import APIClient
import NukeUI
import SwiftUI

/// A player's avatar: their `GET /user/{username}` image once it resolves, initials before
/// that and whenever they have none — mirrors the web client's `PlayerAvatar.vue`.
struct PlayerAvatar: View {
    let username: String
    var size: CGFloat = 36

    @State private var avatarURL: URL?

    var body: some View {
        Circle()
            .fill(Color.accentColor.opacity(0.15))
            .frame(width: size, height: size)
            .overlay { content }
            .clipShape(Circle())
            // Keyed on the username: switching player drops the previous avatar first
            // (below), so a stale image never lingers under a new handle while the new one
            // loads — same guard as the web client's `watch` on `props.username`.
            .task(id: username) {
                avatarURL = nil
                avatarURL = await PlayerAvatarStore.shared.url(for: username)
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
            .foregroundStyle(Color.accentColor)
    }
}

/// Player initials for `PlayerAvatar`. A plain enum, not a `View` member: `View` is
/// `@MainActor`, and this pure string logic is called from off-main contexts (tests).
enum PlayerMonogram {
    /// Up to two letters: the initials of the first two `_`/`.`/`-`/space-separated chunks,
    /// falling back to the first two characters of the raw handle.
    static func initials(from username: String) -> String {
        let words = username.split { !$0.isLetter && !$0.isNumber }
        let letters = words.prefix(2).compactMap(\.first)
        return letters.isEmpty
            ? String(username.prefix(2)).uppercased()
            : String(letters).uppercased()
    }
}

/// Session cache of avatar URLs, keyed by username: one `GET /user/{username}` per player for
/// the life of the process, shared by every `PlayerAvatar` on screen — the collection grid,
/// search results, and a trade's two rails can all name the same player. Mirrors the web
/// client's module-level cache in `useUserAvatar`.
actor PlayerAvatarStore {
    static let shared = PlayerAvatarStore()

    /// `.some(url)` — has an avatar (`url` may itself be `nil`, meaning confirmed none).
    /// Absent — never resolved, or the last attempt failed transiently and is owed a retry.
    private var cached: [String: URL?] = [:]
    private var inFlight: [String: Task<FetchOutcome, Never>] = [:]

    private init() {}

    func url(for username: String) async -> URL? {
        if let cached = cached[username] {
            return cached
        }

        let outcome: FetchOutcome
        if let task = inFlight[username] {
            outcome = await task.value
        } else {
            let task = Task<FetchOutcome, Never> { await Self.fetch(username: username) }
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

    /// A 404 (unknown username) is cached as "no avatar", same as the web client — from an
    /// avatar's point of view the two look identical. Any other failure (network, 5xx, an
    /// expired token) must not freeze the player on initials for the rest of the session, so
    /// it is left uncached for the next `PlayerAvatar` to retry.
    private static func fetch(username: String) async -> FetchOutcome {
        do {
            switch try await APIClientProvider.shared.get_user_profile(path: .init(username: username)) {
            case let .ok(response):
                let profile = try response.body.json
                return .definitive(profile.avatar_url.flatMap(URL.init(string:)))
            case .notFound:
                return .definitive(nil)
            case .unauthorized, .undocumented:
                return .retryLater
            }
        } catch {
            return .retryLater
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
