import ClerkKit
import SwiftUI

/// The Réglages screen's "Compte" section: the avatar, the `@handle` and the Clerk badge.
struct AccountSection: View {
    @Environment(Clerk.self) private var clerk

    var body: some View {
        Section {
            HStack(spacing: 14) {
                AccountAvatar(size: 56)
                VStack(alignment: .leading, spacing: 3) {
                    UsernameLabel(username: clerk.user.map(AccountIdentity.handle(for:)) ?? "—")
                        .font(.headline)
                    HStack(spacing: 4) {
                        Image(systemName: "shield")
                        Text("géré par Clerk")
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
            }
            .padding(.vertical, 4)
        }
    }
}

#Preview {
    List {
        AccountSection()
    }
    .environment(Clerk.shared)
}
