import SwiftUI

/// Why a card cannot be traded right now — shown only on a card locked into an accepted trade.
struct CardReservedBanner: View {
    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: "lock.fill")
                .foregroundStyle(.violetInk)
            VStack(alignment: .leading, spacing: 2) {
                Text("Carte réservée")
                    .fontWeight(.semibold)
                Text(
                    """
                    Engagée dans un échange accepté. Elle ne peut pas être proposée ailleurs tant que \
                    l'échange n'est pas clos ou abandonné.
                    """
                )
                .font(.footnote)
                .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .tintViolet(in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }
}

#Preview {
    CardReservedBanner().padding()
}
