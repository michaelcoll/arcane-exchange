import APIClient
import Foundation
import Testing

@testable import ArcaneExchange

private func profile(avatar: String?) -> Operations.get_user_profile.Output {
    .ok(.init(body: .json(.init(avatar_url: avatar, id: "u1", username: "mizzix_42"))))
}

struct PlayerAvatarStoreTests {
    @Test func resolvesTheAvatarURLOfAPlayer() async {
        let api = FakeAPI()
        api.onGetUserProfile = { _ in profile(avatar: "https://img.example/a.png") }
        let store = PlayerAvatarStore(api: api)

        let url = await store.url(for: "mizzix_42")

        #expect(url == URL(string: "https://img.example/a.png"))
    }

    @Test func asksOncePerPlayer() async {
        let api = FakeAPI()
        api.onGetUserProfile = { _ in profile(avatar: "https://img.example/a.png") }
        let store = PlayerAvatarStore(api: api)

        _ = await store.url(for: "mizzix_42")
        _ = await store.url(for: "mizzix_42")

        #expect(api.calls == ["get_user_profile"])
    }

    @Test func remembersAPlayerWithoutAvatar() async {
        let api = FakeAPI()
        api.onGetUserProfile = { _ in profile(avatar: nil) }
        let store = PlayerAvatarStore(api: api)

        let first = await store.url(for: "mizzix_42")
        _ = await store.url(for: "mizzix_42")

        #expect(first == nil)
        #expect(api.calls == ["get_user_profile"])
    }

    @Test func remembersAnUnknownPlayerAsHavingNoAvatar() async {
        let api = FakeAPI()
        api.onGetUserProfile = { _ in throw APIError(code: "user_not_found", status: 404) }
        let store = PlayerAvatarStore(api: api)

        let first = await store.url(for: "ghost")
        _ = await store.url(for: "ghost")

        #expect(first == nil)
        #expect(api.calls == ["get_user_profile"])
    }

    @Test func retriesAfterATransientFailure() async {
        let api = FakeAPI()
        api.onGetUserProfile = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let store = PlayerAvatarStore(api: api)
        let failed = await store.url(for: "mizzix_42")

        api.onGetUserProfile = { _ in profile(avatar: "https://img.example/a.png") }
        let retried = await store.url(for: "mizzix_42")

        #expect(failed == nil)
        #expect(retried == URL(string: "https://img.example/a.png"))
        #expect(api.calls == ["get_user_profile", "get_user_profile"])
    }

    @Test func doesNotFreezeThePlayerOnAServerError() async {
        let api = FakeAPI()
        api.onGetUserProfile = { _ in throw APIError(code: "internal", status: 500) }
        let store = PlayerAvatarStore(api: api)

        _ = await store.url(for: "mizzix_42")
        _ = await store.url(for: "mizzix_42")

        #expect(api.calls.count == 2)
    }
}
