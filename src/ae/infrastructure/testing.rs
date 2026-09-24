//! `AppState` test harness: every use case defaults to a mock.
//!
//! A test overrides only the field it cares about with struct update syntax:
//! `AppState { get_collection_use_case: Arc::new(mock), ..AppState::for_testing() }`.
//!
//! Most mocks have no expectation and panic if called. Four are pre-armed to succeed on any
//! call: `import_card_use_case`, `edh_rec_caller_adapter`, `auth_service` and
//! `import_price_use_case`. Override them to assert on calls.

use super::AppState;
use crate::application::caller::MockEdhRecCaller;
use crate::application::service::auth_service::MockAuthService;
use crate::application::use_case::{
    MockAbandonTradeUseCase, MockAcceptTradeUseCase, MockAddTradeBinderUseCase,
    MockAddTradeCardUseCase, MockAutocompleteUsersUseCase, MockConfirmTradeUseCase,
    MockCreateTradeUseCase, MockEnqueueCardMarketIdUpdateUseCase,
    MockEnqueueGathererIdUpdateUseCase, MockGetCardImportUseCase, MockGetCardOffersUseCase,
    MockGetCardPriceHistoryUseCase, MockGetCollectionPriceHistoryUseCase,
    MockGetCollectionStatsUseCase, MockGetCollectionUseCase, MockGetCollectionVisibilityUseCase,
    MockGetRarityTradeFiltersUseCase, MockGetSetUseCase, MockGetTradeBindersUseCase,
    MockGetTradeUseCase, MockGetUserProfileUseCase, MockImportCardUseCase, MockImportPriceUseCase,
    MockListSetsUseCase, MockListTradesUseCase, MockRateTradeUseCase, MockRegisterUserUseCase,
    MockRemoveTradeBinderUseCase, MockRemoveTradeCardUseCase, MockSearchCardsUseCase,
    MockSetCollectionVisibilityUseCase, MockSetRarityTradeFilterUseCase, MockStatsUseCase,
};
use crate::domain::card::CardInfo;
use crate::domain::card_import::CardImportId;
use crate::domain::user::User;
use std::sync::Arc;

impl AppState {
    pub(crate) fn for_testing() -> Self {
        let mut mock_import_card = MockImportCardUseCase::new();
        mock_import_card
            .expect_start_import()
            .returning(|_, _| Box::pin(async { Ok(CardImportId::new()) }));

        let mut mock_edh_rec = MockEdhRecCaller::new();
        mock_edh_rec.expect_get_card_info().returning(|_| {
            Box::pin(async {
                Ok(CardInfo {
                    inclusion: 0,
                    total_decks: 0,
                })
            })
        });

        let mut mock_auth = MockAuthService::new();
        mock_auth
            .expect_validate_token()
            .returning(|_| Ok(User::new("test-user-id".to_string(), None, None, None)));

        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));

        Self {
            import_card_use_case: Arc::new(mock_import_card),
            edh_rec_caller_adapter: Arc::new(mock_edh_rec),
            stats_use_case: Arc::new(MockStatsUseCase::new()),
            auth_service: Arc::new(mock_auth),
            get_collection_use_case: Arc::new(MockGetCollectionUseCase::new()),
            search_cards_use_case: Arc::new(MockSearchCardsUseCase::new()),
            import_price_use_case: Arc::new(mock_import_price),
            enqueue_cardmarket_id_use_case: Arc::new(MockEnqueueCardMarketIdUpdateUseCase::new()),
            enqueue_gatherer_id_use_case: Arc::new(MockEnqueueGathererIdUpdateUseCase::new()),
            get_collection_price_history_use_case: Arc::new(
                MockGetCollectionPriceHistoryUseCase::new(),
            ),
            get_card_price_history_use_case: Arc::new(MockGetCardPriceHistoryUseCase::new()),
            get_collection_stats_use_case: Arc::new(MockGetCollectionStatsUseCase::new()),
            register_user_use_case: Arc::new(MockRegisterUserUseCase::new()),
            get_user_profile_use_case: Arc::new(MockGetUserProfileUseCase::new()),
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
            list_sets_use_case: Arc::new(MockListSetsUseCase::new()),
            get_set_use_case: Arc::new(MockGetSetUseCase::new()),
            card_import_query_use_case: Arc::new(MockGetCardImportUseCase::new()),
        }
    }
}
