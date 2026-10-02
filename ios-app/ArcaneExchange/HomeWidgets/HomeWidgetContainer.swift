import SwiftUI
import WidgetKit

/// What every home-screen widget shares: its tile once a snapshot is there, a message while
/// signed out or with nothing to show, and the link the whole widget opens. A widget view only
/// provides its tile and its link.
struct HomeWidgetContainer<Snapshot: Equatable, Tile: View>: View {
    let content: HomeWidgetContent<Snapshot>
    let link: HomeWidgetLink
    @ViewBuilder let tile: (Snapshot) -> Tile

    var body: some View {
        Group {
            switch content {
            case let .snapshot(snapshot):
                tile(snapshot)
            case .signedOut:
                HomeWidgetMessage.signedOut
            case .unavailable:
                HomeWidgetMessage.unavailable
            }
        }
        .widgetURL(link.url)
    }
}

/// The two states a widget can be in without data.
@MainActor
private enum HomeWidgetMessage {
    static var signedOut: some View {
        message("Connecte-toi dans Arcane Exchange", systemImage: "person.crop.circle")
    }

    static var unavailable: some View {
        message("Serveur injoignable pour le moment", systemImage: "wifi.exclamationmark")
    }

    private static func message(_ text: String, systemImage: String) -> some View {
        VStack(spacing: 8) {
            Image(systemName: systemImage)
                .font(.title2)
                .foregroundStyle(Palette.primary)
                .widgetAccentable()
            Text(text)
                .font(.footnote.weight(.medium))
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

#Preview("Déconnecté") {
    HomeWidgetContainer(content: HomeWidgetContent<TradesSnapshot>.signedOut, link: .trades) { _ in
        EmptyView()
    }
    .frame(width: 338, height: 158)
    .background(HomeWidgetSurface.background, in: .rect(cornerRadius: 22))
}

#Preview("Serveur injoignable") {
    HomeWidgetContainer(content: HomeWidgetContent<TradesSnapshot>.unavailable, link: .trades) { _ in
        EmptyView()
    }
    .frame(width: 338, height: 158)
    .background(HomeWidgetSurface.background, in: .rect(cornerRadius: 22))
}
