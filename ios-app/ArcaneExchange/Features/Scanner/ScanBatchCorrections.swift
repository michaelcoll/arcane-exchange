import APIClient
import ArcaneExchangeCore
import SwiftUI

/// The corrections of a scan batch line: its language, its finish, and the way to replace a wrong
/// reading by an impression picked in the catalog.
/// Opened by a tap on its label, the line itself.
struct ScanLineMenu<Label: View>: View {
    let line: ScanBatchLine
    let scanner: ScannerViewModel
    let onReplace: () -> Void
    @ViewBuilder let label: Label

    var body: some View {
        Menu {
            Picker("Langue", selection: Binding(
                get: { line.language },
                set: { scanner.setLanguage($0, ofLine: line.id) }
            )) {
                ForEach(CardLanguage.allCases) { language in
                    Text(language.label).tag(language)
                }
            }
            Toggle("Foil", isOn: Binding(
                get: { line.foil },
                set: { scanner.setFoil($0, ofLine: line.id) }
            ))
            Button("Remplacer la carte…", systemImage: "arrow.triangle.2.circlepath", action: onReplace)
        } label: {
            label
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
        }
        // The line keeps its own colours, not the accent of a button.
        .tint(.primary)
        .foregroundStyle(.primary)
        .accessibilityHint("Corriger la langue, le foil ou la carte")
    }
}

/// The catalog search opened from the scan batch, to replace a line's card or to add a card the
/// camera does not read.
struct ScanCorrectionSheet: View {
    @State private var model: ScanCorrectionViewModel

    init(target: ScanCorrectionTarget, scanner: ScannerViewModel, api: any APIProtocol) {
        _model = State(initialValue: ScanCorrectionViewModel(target: target, scanner: scanner, api: api))
    }

    var body: some View {
        CatalogSearchView(
            search: model.search,
            title: model.target == .add ? "Ajouter au lot" : "Remplacer la carte"
        ) { model.choose($0) }
    }
}
