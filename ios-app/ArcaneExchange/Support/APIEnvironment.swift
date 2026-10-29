import APIClient
import SwiftUI

extension EnvironmentValues {
    /// The API client views hand to their view models. The app sets the session's client, whose
    /// 401 signs the player out. This default, used by previews, never signs anyone out.
    @Entry var api: any APIProtocol = APIClientFactory.preview

    /// The session cache of avatar URLs every `PlayerAvatar` shares. The app sets one built on
    /// the session's client; this default, used by previews, has its own.
    @Entry var avatarStore = PlayerAvatarStore(api: APIClientFactory.preview)

    /// The API client for the public endpoints of the login screen: no token, and a 401 signs
    /// nobody out.
    @Entry var anonymousAPI: any APIProtocol = APIClientFactory.anonymous()
}
