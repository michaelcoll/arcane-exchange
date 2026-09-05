import SwiftUI

/// A player's handle as shown throughout the app: `@` in `.secondary`, the username itself in
/// whatever color the call site inherits. Font/weight modifiers on the view apply to both, same
/// as a plain `Text`.
struct UsernameLabel: View {
    let username: String

    var body: some View {
        Text("\(Text("@").foregroundStyle(.secondary))\(username)")
    }
}

#Preview {
    VStack(alignment: .leading, spacing: 8) {
        UsernameLabel(username: "mizzix_42")
            .font(.headline)
        UsernameLabel(username: "tulipe_arcane")
            .fontWeight(.semibold)
    }
    .padding()
}
