import ClerkKit
import SwiftUI

/// Réglages (`ScrProfile` in the iOS mockup), presented from the nav-bar avatar: the account
/// card, one row per trade rule — each opening its own drawer — and sign-out.
struct AccountSettingsView: View {
    /// The sub-drawers, exactly the mockup's three `ISheet`s.
    private enum Drawer: String, Identifiable {
        case visibility
        case binders
        case rarities

        var id: String {
            rawValue
        }
    }

    @Environment(Clerk.self) private var clerk
    @Environment(\.dismiss) private var dismiss

    @State private var model = AccountSettingsViewModel()
    @State private var drawer: Drawer?
    @State private var isConfirmingSignOut = false

    var body: some View {
        NavigationStack {
            content
                .navigationTitle("Réglages")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        CloseButton { dismiss() }
                    }
                }
                .task { await model.load() }
                .sheet(item: $drawer) { drawer in
                    switch drawer {
                    case .visibility: VisibilitySheet(model: model)
                    case .binders: TradeBindersSheet(model: model)
                    case .rarities: RarityFiltersSheet(model: model)
                    }
                }
                .writeErrorAlert(model)
                .confirmationDialog("Se déconnecter ?", isPresented: $isConfirmingSignOut, titleVisibility: .visible) {
                    Button("Se déconnecter", role: .destructive) {
                        Task { try? await clerk.auth.signOut() }
                    }
                    Button("Annuler", role: .cancel) {}
                }
        }
    }

    private var content: some View {
        List {
            AccountSection()
            // The error goes inline rather than replacing the screen: an expired session fails
            // this load, and "Se déconnecter" is exactly what the user needs to reach then.
            if let error = model.loadError {
                loadErrorSection(error)
            } else {
                tradeRulesSection
            }
            signOutSection
        }
        .refreshable { await model.load() }
        .overlay {
            if model.isLoading {
                ProgressView().controlSize(.large)
            }
        }
    }

    private func loadErrorSection(_ error: AccountSettingsViewModel.RequestError) -> some View {
        Section {
            VStack(alignment: .leading, spacing: 10) {
                Label("Réglages indisponibles", systemImage: "exclamationmark.triangle")
                    .font(.subheadline)
                    .fontWeight(.semibold)
                Text(error.message)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Button("Réessayer") { Task { await model.load() } }
                    .buttonStyle(.bordered)
            }
            .padding(.vertical, 4)
        }
    }

    // MARK: Ce que je propose à l'échange

    /// One row per rule, each opening its drawer — the mockup's `IGroup` of chevron rows, with
    /// the ratio band closing the group. Binders and rarities only bite in `trade` visibility,
    /// so they show up with it, like the web client's `ProfileTradeRules`.
    private var tradeRulesSection: some View {
        Section {
            drawerRow(
                title: "Visibilité de la collection",
                systemImage: "shield",
                value: model.visibility.label,
                drawer: .visibility
            )
            if model.visibility == .trade {
                drawerRow(
                    title: "Classeurs échangeables",
                    systemImage: "rectangle.stack",
                    value: AccountCopy.binderSelection(
                        selected: model.selectedBinders.count,
                        total: model.binders.count
                    ),
                    drawer: .binders
                )
                drawerRow(
                    title: "Filtres de rareté",
                    systemImage: "line.3.horizontal.decrease",
                    value: AccountCopy.openRarities(model.rarities.count(where: \.is_open)),
                    drawer: .rarities
                )
                TradeRatioBand(ratio: model.ratio)
            }
        } header: {
            Text("Ce que je propose à l'échange")
                .sectionCaptionStyle()
        } footer: {
            Text(model.visibility.detail)
        }
    }

    private func drawerRow(title: String, systemImage: String, value: String, drawer: Drawer) -> some View {
        DrawerRow(title: title, systemImage: systemImage, value: value) {
            self.drawer = drawer
        }
    }

    // MARK: Déconnexion

    private var signOutSection: some View {
        Section {
            Button("Se déconnecter", role: .destructive) {
                isConfirmingSignOut = true
            }
            .frame(maxWidth: .infinity)
        } footer: {
            Text("Prix Cardmarket · Données Scryfall · Non affilié à Wizards of the Coast")
                .frame(maxWidth: .infinity)
                .multilineTextAlignment(.center)
        }
    }
}

#Preview {
    AccountSettingsView()
        .environment(Clerk.shared)
}
