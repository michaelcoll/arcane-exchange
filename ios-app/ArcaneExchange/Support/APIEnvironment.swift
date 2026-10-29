import APIClient
import SwiftUI

extension EnvironmentValues {
    /// The API client views hand to their view models. The app sets the session's client, whose
    /// 401 signs the player out. This default, used by previews, never signs anyone out.
    @Entry var api: any APIProtocol = APIClientFactory.authenticated(onUnauthorized: {})
}

extension EnvironmentValues {
    /// The API client for the public endpoints of the login screen: no token, and a 401 signs
    /// nobody out.
    @Entry var anonymousAPI: any APIProtocol = APIClientFactory.anonymous()
}
