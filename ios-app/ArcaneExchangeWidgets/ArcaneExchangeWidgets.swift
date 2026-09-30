import SwiftUI
import WidgetKit

/// The home-screen widgets: the web home page's two tiles, medium size only.
@main
struct ArcaneExchangeWidgets: WidgetBundle {
    var body: some Widget {
        CollectionWidget()
        TradesWidget()
    }
}

struct CollectionWidget: Widget {
    var body: some WidgetConfiguration {
        let provider = HomeWidgetProvider(
            kind: .collection,
            sample: CollectionSnapshot.sample,
            fetch: HomeWidgetFetch.collection,
            nextRefresh: HomeWidgetSchedule.nextCollectionRefresh(after:)
        )
        return StaticConfiguration(kind: HomeWidgetKind.collection.rawValue, provider: provider) { entry in
            CollectionWidgetView(content: entry.content)
                .containerBackground(HomeWidgetSurface.background, for: .widget)
        }
        .configurationDisplayName("Ma collection")
        .description("La valeur de ta collection et son évolution sur 30 jours.")
        .supportedFamilies([.systemMedium])
        // The value graph runs edge to edge, like on the web tile.
        .contentMarginsDisabled()
    }
}

struct TradesWidget: Widget {
    var body: some WidgetConfiguration {
        let provider = HomeWidgetProvider(
            kind: .trades,
            sample: TradesSnapshot.sample,
            fetch: HomeWidgetFetch.trades,
            nextRefresh: HomeWidgetSchedule.nextTradesRefresh(after:)
        )
        return StaticConfiguration(kind: HomeWidgetKind.trades.rawValue, provider: provider) { entry in
            TradesWidgetView(content: entry.content)
                .containerBackground(HomeWidgetSurface.background, for: .widget)
        }
        .configurationDisplayName("Échanges en cours")
        .description("Tes échanges actifs et où ils en sont.")
        .supportedFamilies([.systemMedium])
        .contentMarginsDisabled()
    }
}

struct HomeWidgetEntry<Snapshot: Equatable>: TimelineEntry {
    let date: Date
    let content: HomeWidgetContent<Snapshot>
}

/// One timeline of a single entry: fetched (or recalled) now, refreshed no earlier than
/// `nextRefresh` — iOS picks the actual moment.
struct HomeWidgetProvider<Snapshot: Codable & Equatable & Sendable>: TimelineProvider {
    typealias Entry = HomeWidgetEntry<Snapshot>

    let kind: HomeWidgetKind
    /// The widget gallery's content, and the placeholder while the first timeline loads.
    let sample: Snapshot
    let fetch: @Sendable () async throws -> Snapshot
    let nextRefresh: @Sendable (Date) -> Date

    func placeholder(in _: Context) -> Entry {
        HomeWidgetEntry(date: .now, content: .snapshot(sample))
    }

    func getSnapshot(in context: Context, completion: @escaping @Sendable (Entry) -> Void) {
        guard !context.isPreview else {
            completion(placeholder(in: context))
            return
        }
        Task { await completion(entry()) }
    }

    func getTimeline(in _: Context, completion: @escaping @Sendable (Timeline<Entry>) -> Void) {
        Task {
            let entry = await entry()
            completion(Timeline(entries: [entry], policy: .after(nextRefresh(entry.date))))
        }
    }

    private func entry() async -> Entry {
        let content = await HomeWidgetLoader.load(
            kind,
            cache: HomeWidgetCache(),
            isSignedIn: WidgetSession.isSignedIn(),
            fetch: fetch
        )
        return HomeWidgetEntry(date: .now, content: content)
    }
}
