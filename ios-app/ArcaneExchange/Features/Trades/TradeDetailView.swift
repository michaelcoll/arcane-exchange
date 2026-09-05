import ClerkKit
import SwiftUI

/// The trade screen (`ScrTrade` in the iOS mockup): the two sides of a trade laid out as
/// facing rails, the balance between them, and the one action the current status allows,
/// pinned to the bottom.
///
/// Hides the tab bar: this is a commitment screen (accept, confirm, rate — money and cards
/// change hands), not a place to browse from, and a custom action bar stacked on top of the
/// tab bar read as two unrelated bars glued together. One bottom region, one decision.
struct TradeDetailView: View {
    let route: TradeDetailRoute

    @Environment(Clerk.self) private var clerk

    @State private var model: TradeDetailViewModel
    @State private var confirmation: TradeConfirmation?

    init(route: TradeDetailRoute) {
        self.route = route
        _model = State(initialValue: TradeDetailViewModel(tradeID: route.id))
    }

    var body: some View {
        content
            .navigationTitle("@\(model.partnerUsername.isEmpty ? route.partnerUsername : model.partnerUsername)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { toolbar }
            .toolbar(.hidden, for: .tabBar)
            .task { await model.load() }
            .refreshable { await model.load() }
            .alert(
                confirmation?.title ?? "",
                isPresented: Binding(get: { confirmation != nil }, set: {
                    if !$0 {
                        confirmation = nil
                    }
                }),
                presenting: confirmation,
                actions: { item in
                    Button(item.confirmLabel, role: item.isDestructive ? .destructive : nil) {
                        perform(item)
                    }
                    Button("Annuler", role: .cancel) {}
                },
                message: { Text($0.message) }
            )
            .alert(
                "Action refusée",
                isPresented: Binding(get: { model.actionError != nil }, set: {
                    if !$0 {
                        model.actionError = nil
                    }
                }),
                actions: { Button("OK", role: .cancel) { model.actionError = nil } },
                message: { Text(model.actionError ?? "") }
            )
    }

    @ViewBuilder private var content: some View {
        if model.isLoading, model.trade == nil {
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = model.loadError, model.trade == nil {
            ContentUnavailableView(
                label: { Label(error.title, systemImage: "exclamationmark.triangle") },
                description: { Text(error.message) },
                actions: { Button("Réessayer") { Task { await model.load() } } }
            )
        } else {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    TradeStatusIndicator(
                        status: model.status,
                        stepsRoute: TradeStepsRoute(
                            status: model.status,
                            partnerUsername: model.partnerUsername
                        )
                    )
                    rails
                    if model.status == .completed || model.status == .closed {
                        ratingSection
                    }
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 16)
            }
            .safeAreaInset(edge: .bottom) {
                TradeActionBar(
                    status: model.status,
                    meAccepted: model.meAccepted,
                    meConfirmed: model.meConfirmed,
                    partnerUsername: model.partnerUsername,
                    acceptLabel: acceptLabel,
                    isBusy: model.isBusy,
                    isReady: model.trade != nil,
                    onAccept: { confirmation = .accept(settlement: model.balance.settlementLabel) },
                    onConfirm: { Task { await model.confirmExchange() } }
                )
            }
        }
    }

    @ToolbarContentBuilder private var toolbar: some ToolbarContent {
        ToolbarItem(placement: .topBarTrailing) {
            Menu {
                NavigationLink(value: TradeStepsRoute(status: model.status, partnerUsername: model.partnerUsername)) {
                    Label("Les étapes d'un échange", systemImage: "info.circle")
                }
                if model.status.isOngoing {
                    Button("Abandonner l'échange", systemImage: "xmark", role: .destructive) {
                        confirmation = .abandon
                    }
                }
            } label: {
                Label("Options", systemImage: "ellipsis.circle")
            }
            .disabled(model.trade == nil)
        }
    }

    /// Partner's side on top, mirroring the mockup: it is the side the user composes, so it
    /// sits where the thumb and the eye land first.
    private var rails: some View {
        VStack(alignment: .leading, spacing: 18) {
            TradeCardRail(
                owner: model.partnerUsername,
                cards: model.partnerCards,
                isReserved: model.status.isReserved,
                emptyMessage: "Tu n'as demandé aucune carte pour l'instant.",
                onRemove: removeHandler,
                addSlot: addSlot
            )

            TradeRailPivot(isReserved: model.status.isReserved)

            TradeCardRail(
                owner: clerk.user?.username,
                cards: model.myCards,
                isReserved: model.status.isReserved,
                emptyMessage: "@\(model.partnerUsername) n'a demandé aucune de tes cartes."
            )
        }
    }

    private var removeHandler: ((TradeCard) -> Void)? {
        guard model.status.isEditable else { return nil }
        return { card in removeCard(card) }
    }

    /// The rail's trailing slot, which is how a counter-proposal grows: it opens the
    /// partner's tradable collection, and asking for a card from there lands back on this
    /// trade with the card added.
    private var addSlot: TradeRailAddSlot? {
        guard model.status.isEditable, !model.partnerUsername.isEmpty else { return nil }
        return TradeRailAddSlot(
            label: "Chercher chez lui",
            route: SearchResultsRoute(target: .player(username: model.partnerUsername))
        )
    }

    private var ratingSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Noter @\(model.partnerUsername)")
                .font(.subheadline.weight(.semibold))

            if let rating = model.myRating {
                Label(
                    rating == 0 ? "Notation passée" : "Tu as mis \(rating)/5",
                    systemImage: rating == 0 ? "minus.circle" : "star.fill"
                )
                .font(.footnote.weight(.medium))
                .foregroundStyle(.violet)
            } else {
                TradeRatingStars { rating in Task { await model.rate(rating) } }
                Button("Passer la notation") { Task { await model.rate(0) } }
                    .font(.footnote)
                    .disabled(model.isBusy)
            }

            Text(partnerRatingCaption)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(.quaternary.opacity(0.4), in: .rect(cornerRadius: 16))
    }

    private var partnerRatingCaption: String {
        if let rating = model.partnerRating {
            return rating == 0
                ? "@\(model.partnerUsername) a passé la notation."
                : "@\(model.partnerUsername) t'a mis \(rating)/5."
        }
        return "Optionnel. L'échange se clôture dès que vous avez tous les deux noté ou passé."
    }

    /// "Accepter l'échange", or "Accepter et payer 21 €" when a settlement is owed — the
    /// amount is part of the commitment, so it belongs on the button, not only in the alert.
    private var acceptLabel: String {
        guard let settlement = model.balance.settlementLabel else { return "Accepter l'échange" }
        return "Accepter et \(settlement)"
    }

    /// Removing a card while one party has already accepted restarts the negotiation, so it
    /// gets the same warning as accepting.
    private func removeCard(_ card: TradeCard) {
        if model.status == .oneAccepted {
            confirmation = .modify(card: card)
        } else {
            Task { await model.removePartnerCard(card) }
        }
    }

    private func perform(_ confirmation: TradeConfirmation) {
        Task {
            switch confirmation {
            case .accept: await model.accept()
            case .abandon: await model.abandon()
            case let .modify(card): await model.removePartnerCard(card)
            }
        }
    }
}

/// The confirmations `trade-workflow.instructions.md` requires before a locking or
/// irreversible step.
enum TradeConfirmation {
    /// `settlement` is the "payer 21 €" half of the balance, `nil` when the sides are even —
    /// what the user actually commits to alongside the cards.
    case accept(settlement: String?)
    case abandon
    case modify(card: TradeCard)

    var title: String {
        switch self {
        case .accept: "Accepter cet échange ?"
        case .abandon: "Abandonner l'échange ?"
        case .modify: "Modifier l'échange ?"
        }
    }

    var message: String {
        switch self {
        case let .accept(settlement):
            Self.settlementSentence(settlement) + """
            Les cartes des deux côtés seront réservées. Si l'autre partie modifie ensuite \
            l'échange, il repassera en négociation et devra être accepté à nouveau.
            """
        case .abandon:
            "L'échange sera définitivement abandonné et les cartes réservées libérées. Action irréversible."
        case .modify:
            """
            Une partie a déjà accepté. Modifier libère les cartes réservées, annule les \
            acceptations et relance la négociation.
            """
        }
    }

    /// What the user commits to in cash, spelled out before they lock the trade.
    private static func settlementSentence(_ settlement: String?) -> String {
        guard let settlement else { return "Les valeurs sont équivalentes, aucun règlement. " }
        return "Tu t'engages à \(settlement) en main propre. "
    }

    var confirmLabel: String {
        switch self {
        case .accept: "Accepter"
        case .abandon: "Abandonner"
        case .modify: "Modifier quand même"
        }
    }

    var isDestructive: Bool {
        switch self {
        case .accept: false
        case .abandon, .modify: true
        }
    }
}
