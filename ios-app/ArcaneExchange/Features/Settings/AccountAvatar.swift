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
            .fill(Color.accentColor.opacity(0.15))
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
            .foregroundStyle(Color.accentColor)
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

/// How the account is named on screen, from the Clerk session alone.
enum AccountIdentity {
    /// The `@handle` shown under the avatar: the Clerk username, falling back to the local part
    /// of the primary e-mail address (Clerk instances without usernames still show something).
    static func handle(for user: User) -> String {
        if let username = user.username, !username.isEmpty {
            return username
        }
        guard let email = user.primaryEmailAddress?.emailAddress else { return "—" }
        return String(email.prefix(while: { $0 != "@" }))
    }
}

/// Adds the mockup's nav-bar avatar (`IAvatarBtn`) to a screen inside a `NavigationStack`, and
/// the Réglages sheet it opens. Each tab keeps its own sheet state, like any other nav-bar item.
private struct AccountToolbarModifier: ViewModifier {
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
                AccountSettingsView()
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
