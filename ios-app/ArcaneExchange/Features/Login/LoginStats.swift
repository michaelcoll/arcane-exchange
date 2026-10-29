import APIClient

/// The public stats the login screen shows, from the public `GET /stats`, read before anybody
/// is signed in.
enum LoginStats {
    /// Copies offered for trade across the platform, `nil` when the stats are unavailable
    /// (offline, server error): the screen then simply shows no number.
    static func proposedCopies(api: any APIProtocol) async -> Int? {
        guard let stats = try? await api.get_stats().ok.body.json else { return nil }
        return Int(stats.proposed_copy_number)
    }
}
