import ArcaneExchangeCore
import ClerkKit
import NukeUI
import SwiftUI

/// The signed-in user's Clerk avatar, initials as a fallback.
///
/// Deliberately not `PlayerAvatar`: that one resolves *another* player's image through
/// `GET /user/{username}`, while our own picture is already in the Clerk session — no request.
struct AccountAvatar: View {
    var size: CGFloat = 30

    @Environment(Clerk.self) private var clerk

    var body: some View {
        Circle()
            .fill(Palette.primary.opacity(0.15))
            .frame(width: size, height: size)
            .overlay { content }
            .clipShape(Circle())
    }

    @ViewBuilder private var content: some View {
        if let user = clerk.user, user.hasImage, let url = URL(string: user.imageUrl) {
            LazyImage(url: url) { state in
                if let image = state.image {
                    image.resizable().scaledToFill()
                } else {
                    initials
                }
            }
        } else {
            initials
        }
    }

    private var initials: some View {
        Text(monogram)
            .font(.system(size: size * 0.4, weight: .bold))
            .foregroundStyle(Palette.primary)
    }

    /// The name Clerk knows first, then the handle, then the e-mail — whichever exists.
    private var monogram: String {
        guard let user = clerk.user else { return "?" }
        let named = [user.firstName, user.lastName].compactMap { $0?.first }
        if !named.isEmpty {
            return String(named).uppercased()
        }
        return PlayerMonogram.initials(from: AccountIdentity.handle(for: user))
    }
}

/// Adds the mockup's nav-bar avatar (`IAvatarBtn`) to a screen inside a `NavigationStack`, and
/// the Réglages sheet it opens. Each tab keeps its own sheet state, like any other nav-bar item.
private struct AccountToolbarModifier: ViewModifier {
    @Environment(\.api) private var api
    @State private var isShowingSettings = false

    func body(content: Content) -> some View {
        content
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button(action: { isShowingSettings = true }, label: {
                        AccountAvatar()
                    })
                    // Without this the avatar is drawn as a tinted template image.
                    .buttonStyle(.plain)
                    .accessibilityLabel("Réglages du compte")
                }
            }
            .sheet(isPresented: $isShowingSettings) {
                AccountSettingsView(api: api)
            }
    }
}

extension View {
    /// Goes *inside* the `NavigationStack`, on the screen that owns the navigation title.
    func accountToolbar() -> some View {
        modifier(AccountToolbarModifier())
    }
}

#Preview {
    AccountAvatar(size: 56)
        .environment(Clerk.shared)
}
