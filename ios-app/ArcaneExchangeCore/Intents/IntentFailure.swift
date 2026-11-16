import Foundation

/// Why an intent could not answer, in the words Siri and Shortcuts say to the player.
package enum IntentFailure: Error, Equatable {
    /// Nobody is signed in to the app, or the session expired.
    case signedOut
    /// The Catalogue has no impression for the key.
    case notFound
    /// The call failed: the message is the API error's, in French.
    case unavailable(String)

    package var message: String {
        switch self {
        case .signedOut: "Connecte-toi dans Arcane Exchange."
        case .notFound: "Je ne trouve pas cette carte."
        case let .unavailable(message): message
        }
    }
}

extension IntentFailure: CustomLocalizedStringResourceConvertible {
    package var localizedStringResource: LocalizedStringResource {
        "\(message)"
    }
}

package extension Result where Failure == IntentFailure {
    /// What Siri says: the intent's own words for an answer, the failure's message otherwise.
    /// The one place the failures of every intent are worded.
    func text(_ answer: (Success) -> String) -> String {
        switch self {
        case let .success(value): answer(value)
        case let .failure(failure): failure.message
        }
    }
}
