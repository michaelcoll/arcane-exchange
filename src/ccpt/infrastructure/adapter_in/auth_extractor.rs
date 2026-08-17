use crate::domain::user::User;
use crate::infrastructure::AppState;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
};

#[derive(Debug)]
pub struct AuthenticatedUser(pub User);

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (StatusCode, String);

    #[allow(clippy::manual_async_fn)]
    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let auth_header = parts
                .headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    (
                        StatusCode::UNAUTHORIZED,
                        "Missing Authorization header".to_string(),
                    )
                })?;

            let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid Authorization header format".to_string(),
                )
            })?;

            let user = state
                .auth_service
                .validate_token(token)
                .await
                .map_err(|e| {
                    (
                        StatusCode::UNAUTHORIZED,
                        format!("Authentication failed: {}", e),
                    )
                })?;

            Ok(AuthenticatedUser(user))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::MockEdhRecCaller;
    use crate::application::service::auth_service::{AuthService, MockAuthService};
    use crate::application::use_case::{MockImportCardUseCase, MockStatsUseCase};
    use crate::domain::user::UserId;
    use axum::http::Request;
    use std::sync::Arc;

    fn create_test_app_state_with_auth(auth_service: Arc<dyn AuthService>) -> AppState {
        use crate::application::use_case::{
            MockAbandonTradeUseCase, MockAcceptTradeUseCase, MockAddTradeBinderUseCase,
            MockAddTradeCardUseCase, MockAutocompleteUsersUseCase, MockConfirmTradeUseCase,
            MockCreateTradeUseCase, MockEnqueueCardMarketIdUpdateUseCase,
            MockEnqueueGathererIdUpdateUseCase, MockGetCardOffersUseCase,
            MockGetCardPriceHistoryUseCase, MockGetCollectionPriceHistoryUseCase,
            MockGetCollectionStatsUseCase, MockGetCollectionUseCase,
            MockGetCollectionVisibilityUseCase, MockGetRarityTradeFiltersUseCase,
            MockGetTradeBindersUseCase, MockGetTradeUseCase, MockImportPriceUseCase,
            MockListTradesUseCase, MockRateTradeUseCase, MockRegisterUserUseCase,
            MockRemoveTradeBinderUseCase, MockRemoveTradeCardUseCase, MockSearchCardsUseCase,
            MockSetCollectionVisibilityUseCase, MockSetRarityTradeFilterUseCase,
        };
        AppState {
            import_card_use_case: Arc::new(MockImportCardUseCase::new()),
            edh_rec_caller_adapter: Arc::new(MockEdhRecCaller::new()),
            stats_use_case: Arc::new(MockStatsUseCase::new()),
            auth_service,
            get_collection_use_case: Arc::new(MockGetCollectionUseCase::new()),
            search_cards_use_case: Arc::new(MockSearchCardsUseCase::new()),
            import_price_use_case: Arc::new(MockImportPriceUseCase::new()),
            enqueue_cardmarket_id_use_case: Arc::new(MockEnqueueCardMarketIdUpdateUseCase::new()),
            enqueue_gatherer_id_use_case: Arc::new(MockEnqueueGathererIdUpdateUseCase::new()),
            get_collection_price_history_use_case: Arc::new(
                MockGetCollectionPriceHistoryUseCase::new(),
            ),
            get_card_price_history_use_case: Arc::new(MockGetCardPriceHistoryUseCase::new()),
            get_collection_stats_use_case: Arc::new(MockGetCollectionStatsUseCase::new()),
            register_user_use_case: Arc::new(MockRegisterUserUseCase::new()),
            create_trade_use_case: Arc::new(MockCreateTradeUseCase::new()),
            accept_trade_use_case: Arc::new(MockAcceptTradeUseCase::new()),
            abandon_trade_use_case: Arc::new(MockAbandonTradeUseCase::new()),
            confirm_trade_use_case: Arc::new(MockConfirmTradeUseCase::new()),
            rate_trade_use_case: Arc::new(MockRateTradeUseCase::new()),
            get_card_offers_use_case: Arc::new(MockGetCardOffersUseCase::new()),
            autocomplete_users_use_case: Arc::new(MockAutocompleteUsersUseCase::new()),
            get_trade_use_case: Arc::new(MockGetTradeUseCase::new()),
            list_trades_use_case: Arc::new(MockListTradesUseCase::new()),
            add_trade_card_use_case: Arc::new(MockAddTradeCardUseCase::new()),
            remove_trade_card_use_case: Arc::new(MockRemoveTradeCardUseCase::new()),
            get_collection_visibility_use_case: Arc::new(MockGetCollectionVisibilityUseCase::new()),
            set_collection_visibility_use_case: Arc::new(MockSetCollectionVisibilityUseCase::new()),
            get_trade_binders_use_case: Arc::new(MockGetTradeBindersUseCase::new()),
            add_trade_binder_use_case: Arc::new(MockAddTradeBinderUseCase::new()),
            remove_trade_binder_use_case: Arc::new(MockRemoveTradeBinderUseCase::new()),
            get_rarity_trade_filters_use_case: Arc::new(MockGetRarityTradeFiltersUseCase::new()),
            set_rarity_trade_filter_use_case: Arc::new(MockSetRarityTradeFilterUseCase::new()),
        }
    }

    #[tokio::test]
    async fn missing_authorization_header_returns_unauthorized() {
        let mock_auth = MockAuthService::new();

        let state = create_test_app_state_with_auth(Arc::new(mock_auth));
        let request = Request::builder().body(()).unwrap();
        let (mut parts, _) = request.into_parts();

        let result = AuthenticatedUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
        let (status, message) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(message, "Missing Authorization header");
    }

    #[tokio::test]
    async fn invalid_authorization_format_returns_unauthorized() {
        let mock_auth = MockAuthService::new();

        let state = create_test_app_state_with_auth(Arc::new(mock_auth));
        let request = Request::builder()
            .header(header::AUTHORIZATION, "Basic invalid")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let result = AuthenticatedUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
        let (status, message) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(message, "Invalid Authorization header format");
    }

    #[tokio::test]
    async fn valid_bearer_token_returns_user() {
        let expected_user = User::new("user_clerk123".to_string(), None, None);

        let mut mock_auth = MockAuthService::new();
        mock_auth
            .expect_validate_token()
            .with(mockall::predicate::eq("valid-token"))
            .returning(move |_| Ok(expected_user.clone()));

        let state = create_test_app_state_with_auth(Arc::new(mock_auth));
        let request = Request::builder()
            .header(header::AUTHORIZATION, "Bearer valid-token")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let result = AuthenticatedUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let AuthenticatedUser(user) = result.unwrap();
        assert_eq!(user.id, UserId::new("user_clerk123"));
        assert_eq!(user.name, None);
    }
}
