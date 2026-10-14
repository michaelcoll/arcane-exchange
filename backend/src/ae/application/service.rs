pub mod autocomplete_user_service;
pub mod card_offer_service;
pub mod card_price_history_service;
pub mod collection;
pub mod get_user_profile_service;
pub mod maintenance;
pub mod register_user_service;
pub mod search;
pub mod sets;
pub mod showcase_service;
pub mod stats_service;
pub mod trade;
pub mod trade_settings;

#[cfg(test)]
mod pagination_frontend_sync_tests {
    use crate::application::service::collection::get::COLLECTION_MAX_OFFSET;
    use crate::application::service::search::search_cards::SEARCH_MAX_OFFSET;
    use crate::application::service::trade::list::TRADES_MAX_OFFSET;

    // The frontend stops paginating before hitting these limits, so it duplicates them in
    // TypeScript. This test guards against the two drifting apart silently.
    const FRONTEND_PAGINATION_PATH: &str = "frontend/app/utils/pagination.ts";
    // `include_str!` only takes a literal: keep it in sync with `FRONTEND_PAGINATION_PATH`.
    const FRONTEND_PAGINATION_TS: &str =
        include_str!("../../../../frontend/app/utils/pagination.ts");

    fn extract_const(name: &str) -> u32 {
        let needle = format!("export const {name} = ");
        let start = FRONTEND_PAGINATION_TS
            .find(&needle)
            .unwrap_or_else(|| panic!("`{name}` not found in {FRONTEND_PAGINATION_PATH}"))
            + needle.len();
        let rest = &FRONTEND_PAGINATION_TS[start..];
        let end = rest
            .find(';')
            .expect("expected a `;` after the constant value");
        rest[..end].replace('_', "").parse().unwrap_or_else(|e| {
            panic!("failed to parse `{name}` value from {FRONTEND_PAGINATION_PATH}: {e}")
        })
    }

    #[test]
    fn frontend_offset_limits_match_the_backend_constants() {
        for (name, backend_value) in [
            ("COLLECTION_MAX_OFFSET", COLLECTION_MAX_OFFSET),
            ("SEARCH_MAX_OFFSET", SEARCH_MAX_OFFSET),
            ("TRADES_MAX_OFFSET", TRADES_MAX_OFFSET),
        ] {
            assert_eq!(
                extract_const(name),
                backend_value,
                "{name} drifted between the backend and {FRONTEND_PAGINATION_PATH}"
            );
        }
    }
}
