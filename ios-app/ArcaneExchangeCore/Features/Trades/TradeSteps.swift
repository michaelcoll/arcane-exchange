/// The five lifecycle steps, spelled out for `TradeStepsView`.
///
/// A plain enum rather than members of the view: `View` is `@MainActor`, and the tests read
/// this list from an off-main context.
package enum TradeSteps {
    package struct Step {
        package let title: String
        package let detail: String
    }

    /// One entry per `TradeStatus.lifecycle` step, in the same order.
    package static let all: [Step] = [
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
