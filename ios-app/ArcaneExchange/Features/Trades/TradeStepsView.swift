import SwiftUI

struct TradeStepsRoute: Hashable {
    let status: TradeStatus
    let partnerUsername: String
}

/// "Les étapes d'un échange" (`ScrSteps` in the mockup): the five states of the trade state
/// machine, spelled out, with the current one called out.
///
/// It exists because the rules are not guessable from the screen — an acceptance reserving
/// cards and abandoning competing trades is the kind of thing a user needs told once.
///
/// Deliberately not a `List`: the steps are one connected timeline, and row separators plus
/// list insets would cut the rail that ties the nodes together.
struct TradeStepsView: View {
    let route: TradeStepsRoute

    /// `nil` for an abandoned trade, which left the nominal path — every step then reads as
    /// "not reached", none as current.
    private var currentIndex: Int? {
        route.status.lifecycleIndex
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                Text(intro)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)

                timeline

                if route.status == .abandoned {
                    Label("Les cartes réservées ont été libérées.", systemImage: "lock.open")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 16)
        }
        .navigationTitle("Les étapes d'un échange")
        .navigationBarTitleDisplayMode(.inline)
    }

    private var intro: String {
        guard let currentIndex else {
            return "Cet échange a été abandonné avant son terme. Voici les cinq étapes qu'il aurait traversées."
        }
        let partner = route.partnerUsername.isEmpty ? "un autre joueur" : "@\(route.partnerUsername)"
        return "Un échange avec \(partner) traverse cinq étapes. Tu es à l'étape \(currentIndex + 1)."
    }

    private var timeline: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(TradeSteps.all.enumerated()), id: \.offset) { index, step in
                stepRow(index: index, step: step, isLast: index == TradeSteps.all.count - 1)
            }
        }
    }

    /// One step: the rail on the left carries the node and the segment down to the next one,
    /// so the height of the text is what stretches the line.
    private func stepRow(index: Int, step: TradeSteps.Step, isLast: Bool) -> some View {
        let state = TradeStepState(index: index, currentIndex: currentIndex)

        return HStack(alignment: .top, spacing: 13) {
            VStack(spacing: 3) {
                TradeStepNode(index: index, state: state, status: route.status)
                if !isLast {
                    Capsule()
                        .fill(state == .done ? Color.green.opacity(0.45) : Color.secondary.opacity(0.2))
                        .frame(width: 2)
                        .frame(maxHeight: .infinity)
                }
            }
            .frame(width: TradeStepNode.size)

            VStack(alignment: .leading, spacing: 4) {
                Text(step.title)
                    .font(.callout.weight(.semibold))
                    .foregroundStyle(state == .upcoming ? .secondary : .primary)
                Text(step.detail)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .opacity(state == .upcoming ? 0.7 : 1)
                    .fixedSize(horizontal: false, vertical: true)
                if state == .current {
                    TradeCurrentStepBadge(tint: route.status.tint)
                        .padding(.top, 3)
                }
            }
            .padding(.bottom, isLast ? 0 : 18)
            // The node is 26pt tall against a ~20pt line of text: nudging the text down lines
            // its cap height up with the middle of the node.
            .padding(.top, 2)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

/// Where a step sits relative to the trade's own position.
enum TradeStepState {
    case done
    case current
    case upcoming

    init(index: Int, currentIndex: Int?) {
        guard let currentIndex else {
            self = .upcoming
            return
        }
        self = index < currentIndex ? .done : (index == currentIndex ? .current : .upcoming)
    }
}

/// The numbered dot on the rail: a green tick once passed, a filled disc with a halo for the
/// step in progress, an outlined number for the ones still ahead.
struct TradeStepNode: View {
    static let size: CGFloat = 26

    let index: Int
    let state: TradeStepState
    /// Only the current node is tinted; the trade's status is what tints it.
    let status: TradeStatus

    var body: some View {
        glyph
            .font(.system(size: 11, weight: .bold, design: .monospaced))
            .foregroundStyle(foreground)
            .frame(width: Self.size, height: Self.size)
            .background(background, in: .circle)
            .overlay { Circle().strokeBorder(border, lineWidth: 1) }
            // The mockup's `box-shadow: 0 0 0 4px` — a soft ring outside the disc, so the live
            // step reads at a glance without being any bigger than the others.
            .overlay {
                if state == .current {
                    Circle()
                        .strokeBorder(status.tint.opacity(0.22), lineWidth: 4)
                        .padding(-4)
                }
            }
    }

    @ViewBuilder private var glyph: some View {
        switch state {
        case .done: Image(systemName: "checkmark")
        case .current, .upcoming: Text("\(index + 1)")
        }
    }

    private var background: Color {
        switch state {
        case .done: .green.opacity(0.15)
        case .current: status.tint
        case .upcoming: .secondary.opacity(0.1)
        }
    }

    private var border: Color {
        switch state {
        case .done: .green.opacity(0.4)
        case .current: status.tint
        case .upcoming: .secondary.opacity(0.25)
        }
    }

    private var foreground: Color {
        switch state {
        case .done: .green
        case .current: status.onTint
        case .upcoming: .secondary
        }
    }
}

/// "ÉTAPE EN COURS" — the small monospaced capsule the mockup pins under the live step.
struct TradeCurrentStepBadge: View {
    let tint: Color

    var body: some View {
        Label("étape en cours", systemImage: "clock")
            .font(.system(size: 10, weight: .semibold, design: .monospaced))
            .textCase(.uppercase)
            .tracking(1)
            .foregroundStyle(tint)
            .padding(.horizontal, 9)
            .padding(.vertical, 4)
            .background(tint.opacity(0.12), in: .capsule)
            .overlay { Capsule().strokeBorder(tint.opacity(0.3), lineWidth: 1) }
    }
}

/// The five lifecycle steps, spelled out for `TradeStepsView`.
///
/// A plain enum rather than members of the view: `View` is `@MainActor`, and the tests read
/// this list from an off-main context.
enum TradeSteps {
    struct Step {
        let title: String
        let detail: String
    }

    /// One entry per `TradeStatus.lifecycle` step, in the same order.
    static let all: [Step] = [
        Step(
            title: "Négociation",
            detail: """
            Tu composes ta demande en piochant dans la collection de l'autre joueur ; \
            lui compose la sienne dans la tienne. Chaque modification est notifiée.
            """
        ),
        Step(
            title: "1 acceptation",
            detail: """
            Dès qu'un joueur accepte, les cartes des deux côtés sont réservées et les autres \
            échanges qui les impliquent sont abandonnés.
            """
        ),
        Step(
            title: "Verrouillé",
            detail: """
            Les deux ont accepté. Rendez-vous en main propre pour échanger les cartes et \
            régler l'écart de valeur.
            """
        ),
        Step(
            title: "Échange réalisé",
            detail: """
            Chacun confirme de son côté que l'échange a bien eu lieu. Les cartes changent \
            alors de collection.
            """
        ),
        Step(
            title: "Clôturé",
            detail: """
            Vous pouvez vous noter mutuellement. Une fois les deux notes posées ou passées, \
            l'échange est archivé.
            """
        )
    ]
}

#Preview {
    NavigationStack {
        TradeStepsView(route: TradeStepsRoute(status: .oneAccepted, partnerUsername: "mizzix_42"))
    }
}
