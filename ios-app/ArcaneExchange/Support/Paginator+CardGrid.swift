import Foundation

extension Paginator where Item == CollectionCard {
    /// A paginator for a grid of cards: each applied page warms the artwork just below the fold,
    /// and a reload drops whatever was still queued for the previous result set.
    ///
    /// `fetch` receives `owner` back, so the paginator never keeps its owner alive; once the
    /// owner is gone the fetch is cancelled rather than answered with an empty page.
    static func cardGrid<Owner: AnyObject>(
        owner: Owner,
        fetch: @escaping @MainActor (_ owner: Owner, _ page: Int32, _ pageSize: Int32) async throws
            -> PaginatorPage<CollectionCard>
    ) -> Paginator {
        Paginator(
            fetch: { [weak owner] page, pageSize in
                guard let owner else { throw CancellationError() }
                return try await fetch(owner, page, pageSize)
            },
            // The page doubles as the prefetch window: the next page is asked four cards from
            // the end, so the twenty that come back are the ones just below the fold.
            onPageApplied: { ArtworkPipeline.prefetch(CardArtwork.urls(for: $0)) },
            onReload: { ArtworkPipeline.cancelPrefetching() }
        )
    }
}
