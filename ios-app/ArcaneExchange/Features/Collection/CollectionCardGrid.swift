import SwiftUI

/// The collection's main scrollable content once cards are loaded: filter rail, summary line,
/// and the card grid itself, with a "load more" spinner at the bottom.
struct CollectionCardGrid: View {
    let cards: [CollectionCard]
    let isLoadingMore: Bool
    let summary: String
    @Binding var filters: CollectionFilters
    let cardTransition: Namespace.ID
    let onFilterTap: () -> Void
    let onCardAppear: (CollectionCard) async -> Void

    private let columns = [GridItem(.adaptive(minimum: 140), spacing: 14)]

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 14) {
                CollectionFilterRail(filters: $filters, onFilterTap: onFilterTap)

                Text(summary)
                    .font(.caption)
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)

                LazyVGrid(columns: columns, spacing: 18) {
                    ForEach(cards, id: \.self) { card in
                        NavigationLink(value: CardDetailRoute(card: card)) {
                            CollectionCardCell(card: card)
                        }
                        .buttonStyle(.plain)
                        .matchedTransitionSource(id: card.scryfall_id, in: cardTransition)
                        .task { await onCardAppear(card) }
                    }
                }

                if isLoadingMore {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 24)
        }
    }
}

#Preview {
    @Previewable @Namespace var cardTransition
    NavigationStack {
        CollectionCardGrid(
            cards: CollectionCard.previewGrid,
            isLoadingMore: true,
            summary: "42 cartes · triées par prix",
            filters: .constant(CollectionFilters()),
            cardTransition: cardTransition,
            onFilterTap: {},
            onCardAppear: { _ in }
        )
    }
}
