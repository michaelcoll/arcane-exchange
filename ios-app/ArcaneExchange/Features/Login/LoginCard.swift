import ClerkKit
import ClerkKitUI
import SwiftUI

/// The glass card of the login screen (`.aw-sheet` in the iOS mockup): the brand, the tagline
/// and one button opening Clerk's prebuilt auth view as a sheet — sign-in and sign-up, with
/// whatever methods the Clerk instance enables.
///
/// The auth view is a sheet rather than embedded in the card: it is a `NavigationStack`
/// that paints an opaque system background whatever `clerkTheme.colors.background` says, so
/// the card's glass cannot show through it.
struct LoginCard: View {
    let tagline: String

    @State private var authIsPresented = false

    private static let shape = RoundedRectangle(cornerRadius: 32, style: .continuous)

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            LoginBrand()
            Text(tagline)
                .font(.title2)
                .fontWeight(.semibold)
                .fixedSize(horizontal: false, vertical: true)
            Button {
                authIsPresented = true
            } label: {
                Text("Se connecter")
                    .fontWeight(.semibold)
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .buttonBorderShape(.capsule)
            .controlSize(.large)
            .foregroundStyle(Palette.onPrimary)
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.regularMaterial, in: Self.shape)
        .overlay { Self.shape.strokeBorder(.primary.opacity(0.12), lineWidth: 0.5) }
        // Dismissible: the sheet closes itself once the player is signed in.
        .sheet(isPresented: $authIsPresented) {
            AuthView()
                // The brand, not the logo configured on the Clerk dashboard.
                .clerkAppIconView { LoginBrand().padding(.bottom, 24) }
                .environment(\.clerkTheme, .login)
        }
    }
}

/// The two interlocked diamonds and the name, as in the web header.
private struct LoginBrand: View {
    var body: some View {
        HStack(spacing: 10) {
            ZStack {
                diamond.foregroundStyle(Palette.primary).offset(x: -3.5)
                diamond.foregroundStyle(Palette.secondary).offset(x: 3.5)
            }
            .frame(width: 28, height: 28)
            .accessibilityHidden(true)

            Text("Arcane ").fontWeight(.medium) + Text("Exchange").fontWeight(.bold)
        }
        .font(.title3)
    }

    private var diamond: some View {
        RoundedRectangle(cornerRadius: 3, style: .continuous)
            .strokeBorder(lineWidth: 2)
            .frame(width: 11, height: 11)
            .rotationEffect(.degrees(45))
    }
}

private extension ClerkTheme {
    /// Clerk's views with the app's accent. Both colours are light/dark assets and the rest are
    /// Clerk's own adaptive defaults: the theme follows the system appearance on its own.
    static let login = ClerkTheme(
        colors: .init(
            primary: Palette.primary,
            primaryForeground: Palette.onPrimary,
            ring: Palette.primary
        )
    )
}

#Preview {
    LoginCard(tagline: LoginCopy.tagline(proposedCopies: 1248))
        .padding(10)
        .environment(Clerk.shared)
}
