use super::ErrorResponse;
use super::autocomplete::dto::UserSuggestionResponse;
use super::card::dto::{
    CardOfferResponse, CardOffersSortByParam, PaginatedCardOffersResponse,
    PriceHistoryEntryResponse,
};
use super::collection::dto::{
    BinderInfoResponse, CardImportLineErrorResponse, CardImportResponse, CardImportStartedResponse,
    CollectionCardResponse, CollectionStatsResponse, MessageResponse, PaginatedCollectionResponse,
    PriceGuideResponse, RarityCodeParam, SetInfoResponse, SortByParam, SortDirParam,
};
use super::maintenance::dto::EnqueueResponse;
use super::stats::dto::StatsResponse;
use super::trade::dto::{
    AddTradeCardRequest, CreateTradeRequest, CreateTradeResponse, PaginatedTradesResponse,
    RateTradeRequest, RemoveTradeCardRequest, TradeCardResponse, TradeDetailResponse,
    TradePartyStateResponse, TradeStatusParam, TradeSummaryResponse,
};
use super::trade_settings::dto::{
    AddTradeBinderRequest, CollectionVisibilityParam, RarityFilterResponse, RarityFiltersResponse,
    SetRarityFilterRequest, SetVisibilityRequest, TradeBindersResponse, VisibilityResponse,
};
use super::user::dto::UserProfileResponse;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::collection::controller::get_collection,
        super::collection::controller::import_cards,
        super::collection::controller::list_card_imports,
        super::collection::controller::get_card_import,
        super::collection::controller::get_collection_stats,
        super::collection::controller::get_collection_price_history,
        super::trade_settings::controller::get_trade_visibility,
        super::trade_settings::controller::set_trade_visibility,
        super::trade_settings::controller::get_trade_binders,
        super::trade_settings::controller::add_trade_binder,
        super::trade_settings::controller::remove_trade_binder,
        super::trade_settings::controller::get_trade_rarity_filters,
        super::trade_settings::controller::set_trade_rarity_filter,
        super::search::controller::search_cards,
        super::card::controller::get_card_info,
        super::card::controller::get_card_price_history,
        super::card::controller::get_card_offers,
        super::stats::controller::get_stats,
        super::maintenance::controller::trigger_price_update,
        super::maintenance::controller::update_cardmarket_ids,
        super::user::controller::register,
        super::user::controller::get_user_profile,
        super::trade::controller::create_trade,
        super::trade::controller::add_trade_card,
        super::trade::controller::remove_trade_card,
        super::trade::controller::accept_trade,
        super::trade::controller::abandon_trade,
        super::trade::controller::confirm_trade,
        super::trade::controller::rate_trade,
        super::trade::controller::get_trade,
        super::trade::controller::list_trades,
        super::autocomplete::controller::autocomplete_user,
        super::sets::controller::list_sets,
        super::sets::controller::get_set,
    ),
    components(schemas(
        PriceGuideResponse,
        CollectionCardResponse,
        MessageResponse,
        CardImportStartedResponse,
        CardImportResponse,
        CardImportLineErrorResponse,
        PaginatedCollectionResponse,
        PriceHistoryEntryResponse,
        SortByParam,
        SortDirParam,
        RarityCodeParam,
        CollectionStatsResponse,
        SetInfoResponse,
        BinderInfoResponse,
        RarityFilterResponse,
        RarityFiltersResponse,
        SetRarityFilterRequest,
        StatsResponse,
        EnqueueResponse,
        CreateTradeRequest,
        CreateTradeResponse,
        AddTradeCardRequest,
        RemoveTradeCardRequest,
        RateTradeRequest,
        CardOfferResponse,
        PaginatedCardOffersResponse,
        CardOffersSortByParam,
        UserSuggestionResponse,
        TradeCardResponse,
        TradePartyStateResponse,
        TradeDetailResponse,
        TradeSummaryResponse,
        PaginatedTradesResponse,
        TradeStatusParam,
        VisibilityResponse,
        SetVisibilityRequest,
        CollectionVisibilityParam,
        TradeBindersResponse,
        AddTradeBinderRequest,
        UserProfileResponse,
        ErrorResponse,
    )),
    modifiers(&SecurityAddon, &ErrorResponseAddon),
    info(
        title = "Card Collection Price Tracker API",
        version = "0.1.0",
        description = "REST API for tracking Magic: The Gathering card prices",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT")
    ),
    tags(
        (name = "card", description = "Single card lookup, price history and sale offers (authentication required)"),
        (name = "collection", description = "Player's private collection (authentication required, no public catalog)"),
        (name = "trade-settings", description = "Settings deciding what of a player's collection is exposed for trade: visibility, binders open for trade, rarity filters (authentication required)"),
        (name = "search", description = "Public card search across all users' collections (authentication required)"),
        (name = "maintenance", description = "Maintenance operations (public)"),
        (name = "auth", description = "Authentication and user registration (authentication required)"),
        (name = "trades", description = "Trade requests between two collectors (authentication required)"),
        (name = "autocomplete", description = "Public username autocomplete (no authentication)"),
        (name = "sets", description = "Set catalog lookup (no authentication)"),
        (name = "stats", description = "Global platform statistics (no authentication)"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Every error response has the `ErrorResponse` body — built by `AppError::into_response`, or by
/// `with_error_body` for axum's own rejections — so it is attached here once rather than on
/// each `utoipa::path`.
struct ErrorResponseAddon;

impl utoipa::Modify for ErrorResponseAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::{Content, Ref, RefOr};

        for item in openapi.paths.paths.values_mut() {
            let operations = [
                &mut item.get,
                &mut item.put,
                &mut item.post,
                &mut item.delete,
                &mut item.patch,
            ];
            for operation in operations.into_iter().flatten() {
                for (status, response) in operation.responses.responses.iter_mut() {
                    let RefOr::T(response) = response else {
                        continue;
                    };
                    if status.starts_with('4') || status.starts_with('5') {
                        response.content.insert(
                            "application/json".to_string(),
                            Content::new(Some(Ref::from_schema_name("ErrorResponse"))).into(),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::openapi::RefOr;

    fn json_schema_of(status: &str, path: &str) -> Option<String> {
        let openapi = ApiDoc::openapi();
        let operation = openapi.paths.paths[path].post.clone().unwrap();
        let RefOr::T(response) = &operation.responses.responses[status] else {
            panic!("inline response expected");
        };
        let content = response.content.get("application/json")?;
        let RefOr::T(content) = content else {
            panic!("inline content expected");
        };
        match content.schema.as_ref()? {
            RefOr::Ref(r) => Some(r.ref_location.clone()),
            RefOr::T(_) => None,
        }
    }

    #[test]
    fn error_responses_document_the_error_body() {
        for status in ["400", "401", "409"] {
            assert_eq!(
                json_schema_of(status, "/collection/import").as_deref(),
                Some("#/components/schemas/ErrorResponse"),
                "status {status}"
            );
        }
    }

    #[test]
    fn trade_settings_live_under_the_collection_with_their_own_tag() {
        let openapi = ApiDoc::openapi();
        let paths = &openapi.paths.paths;

        for path in [
            "/collection/trade-settings/visibility",
            "/collection/trade-settings/binders",
            "/collection/trade-settings/binders/{name}",
            "/collection/trade-settings/rarities",
        ] {
            let item = paths.get(path).unwrap_or_else(|| panic!("{path} missing"));
            let operations = [&item.get, &item.put, &item.post, &item.delete];
            for operation in operations.into_iter().flatten() {
                assert_eq!(
                    operation.tags.as_deref(),
                    Some(&["trade-settings".to_string()][..]),
                    "{path}"
                );
            }
        }

        for path in [
            "/user/visibility",
            "/user/trade-binders",
            "/user/trade-binders/{name}",
            "/collection/visibility/rarities",
        ] {
            assert!(!paths.contains_key(path), "{path} should be gone");
        }
    }

    #[test]
    fn success_responses_keep_their_own_body() {
        assert_eq!(
            json_schema_of("202", "/collection/import").as_deref(),
            Some("#/components/schemas/CardImportStartedResponse")
        );
    }
}
