import ClerkKit

/// How the account is named on screen, from the Clerk session alone.
package enum AccountIdentity {
    /// The `@handle` shown under the avatar: the Clerk username, falling back to the local part
    /// of the primary e-mail address (Clerk instances without usernames still show something).
    package static func handle(for user: User) -> String {
        if let username = user.username, !username.isEmpty {
            return username
        }
        guard let email = user.primaryEmailAddress?.emailAddress else { return "—" }
        return String(email.prefix(while: { $0 != "@" }))
    }
}
