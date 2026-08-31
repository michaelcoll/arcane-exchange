import SwiftUI

/// Circular monogram for a player, derived from the username — the app has no avatar images.
struct PlayerAvatar: View {
    let username: String
    var size: CGFloat = 36

    var body: some View {
        Circle()
            .fill(Color.accentColor.opacity(0.15))
            .frame(width: size, height: size)
            .overlay {
                Text(PlayerMonogram.initials(from: username))
                    .font(.system(size: size * 0.4, weight: .bold))
                    .foregroundStyle(Color.accentColor)
            }
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

#Preview {
    HStack {
        PlayerAvatar(username: "mizzix_42")
        PlayerAvatar(username: "golgari.jo", size: 48)
        PlayerAvatar(username: "x")
    }
    .padding()
}
