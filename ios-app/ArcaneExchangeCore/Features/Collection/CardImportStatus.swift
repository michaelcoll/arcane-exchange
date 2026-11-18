import APIClient

package extension Components.Schemas.CardImportResponse {
    /// Whether the import ran to the end without failing.
    var isCompleted: Bool {
        status == "completed"
    }
}
