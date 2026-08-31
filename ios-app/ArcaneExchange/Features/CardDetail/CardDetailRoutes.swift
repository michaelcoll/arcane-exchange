/// Routes pushed onto whichever `NavigationStack` shows a card list — Collection today,
/// Search once it exists. Both carry the whole card: the list already holds it, so the
/// detail and owners screens open without another round-trip.
struct CardDetailRoute: Hashable {
    let card: CollectionCard
}

struct CardOffersRoute: Hashable {
    let card: CollectionCard
}
