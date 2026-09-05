pub mod auth_service;
pub mod autocomplete_user_service;
pub mod card_collection_service;
pub mod card_offer_service;
pub mod card_price_history_service;
pub mod cardmarket_id_enqueue_service;
pub mod collection_price_history_service;
pub mod collection_service;
pub mod collection_stats_service;
pub mod collection_visibility_service;
pub mod gatherer_id_enqueue_service;
pub mod get_user_profile_service;
pub mod import_card_service;
pub mod import_price_service;
mod parse_service;
pub mod rarity_trade_filter_service;
pub mod register_user_service;
pub mod search_service;
pub mod set_service;
pub mod stats_service;
pub mod trade_binder_service;
pub mod trade_service;
pub mod update_card_market_service;
pub mod update_gatherer_service;

#[cfg(test)]
mod pagination_frontend_sync_tests {
    use crate::application::service::collection_service::COLLECTION_MAX_OFFSET;
    use crate::application::service::search_service::SEARCH_MAX_OFFSET;
    use crate::application::service::trade_service::TRADES_MAX_OFFSET;

    // The frontend stops paginating before hitting these limits (see
    // `frontend-vue/app/utils/pagination.ts`), so it duplicates them in TypeScript. This test
    // guards against the two drifting apart silently.
    const FRONTEND_PAGINATION_TS: &str =
        include_str!("../../../frontend-vue/app/utils/pagination.ts");

    fn extract_const(name: &str) -> u32 {
        let needle = format!("export const {name} = ");
        let start = FRONTEND_PAGINATION_TS.find(&needle).unwrap_or_else(|| {
            panic!("`{name}` not found in frontend-vue/app/utils/pagination.ts")
        }) + needle.len();
        let rest = &FRONTEND_PAGINATION_TS[start..];
        let end = rest
            .find(';')
            .expect("expected a `;` after the constant value");
        rest[..end].replace('_', "").parse().unwrap_or_else(|e| {
            panic!("failed to parse `{name}` value from frontend-vue/app/utils/pagination.ts: {e}")
        })
    }

    #[test]
    fn frontend_offset_limits_match_the_backend_constants() {
        assert_eq!(
            extract_const("COLLECTION_MAX_OFFSET"),
            COLLECTION_MAX_OFFSET,
            "COLLECTION_MAX_OFFSET drifted between the backend and frontend-vue/app/utils/pagination.ts"
        );
        assert_eq!(
            extract_const("SEARCH_MAX_OFFSET"),
            SEARCH_MAX_OFFSET,
            "SEARCH_MAX_OFFSET drifted between the backend and frontend-vue/app/utils/pagination.ts"
        );
        assert_eq!(
            extract_const("TRADES_MAX_OFFSET"),
            TRADES_MAX_OFFSET,
            "TRADES_MAX_OFFSET drifted between the backend and frontend-vue/app/utils/pagination.ts"
        );
    }
}
