import ClerkKit
import Testing

@testable import ArcaneExchangeCore

struct AccountIdentityTests {
    @Test func showsTheClerkUsername() {
        var user = User.mock
        user.username = "jace"
        #expect(AccountIdentity.handle(for: user) == "jace")
    }

    @Test func fallsBackToTheLocalPartOfThePrimaryEmail() {
        var user = User.mock
        user.username = nil
        #expect(AccountIdentity.handle(for: user) == "user")
    }

    @Test func treatsABlankUsernameAsMissing() {
        var user = User.mock
        user.username = ""
        #expect(AccountIdentity.handle(for: user) == "user")
    }

    @Test func showsADashWithNeitherUsernameNorEmail() {
        var user = User.mock
        user.username = nil
        user.primaryEmailAddressId = nil
        #expect(AccountIdentity.handle(for: user) == "—")
    }
}
