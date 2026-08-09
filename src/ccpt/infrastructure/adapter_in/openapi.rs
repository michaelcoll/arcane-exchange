use super::autocomplete::dto::UserSuggestionResponse;
use super::card::dto::{
    CardOfferResponse, CardOffersSortByParam, PaginatedCardOffersResponse,
    PriceHistoryEntryResponse,
};
use super::collection::dto::{
    CollectionCardResponse, CollectionStatsResponse, MessageResponse, PaginatedCollectionResponse,
    PriceGuideResponse, RarityCodeParam, SetInfoResponse, SortByParam, SortDirParam,
};
use super::maintenance::dto::{EnqueueResponse, StatsResponse};
use super::trade::dto::{
    CreateTradeRequest, PaginatedTradesResponse, RateTradeRequest, TradeCardResponse,
    TradeDetailResponse, TradePartyStateResponse, TradeStatusParam, TradeSummaryResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::collection::controller::get_collection,
        super::collection::controller::import_cards,
        super::collection::controller::get_collection_stats,
        super::collection::controller::get_collection_price_history,
        super::search::controller::search_cards,
        super::card::controller::get_card_info,
        super::card::controller::get_card_price_history,
        super::card::controller::get_card_offers,
        super::maintenance::controller::get_stats,
        super::maintenance::controller::trigger_price_update,
        super::maintenance::controller::update_cardmarket_ids,
        super::user::controller::register,
        super::trade::controller::create_trade,
        super::trade::controller::accept_trade,
        super::trade::controller::abandon_trade,
        super::trade::controller::confirm_trade,
        super::trade::controller::rate_trade,
        super::trade::controller::get_trade,
        super::trade::controller::list_trades,
        super::autocomplete::controller::autocomplete_user,
    ),
    components(schemas(
        PriceGuideResponse,
        CollectionCardResponse,
        MessageResponse,
        PaginatedCollectionResponse,
        PriceHistoryEntryResponse,
        SortByParam,
        SortDirParam,
        RarityCodeParam,
        CollectionStatsResponse,
        SetInfoResponse,
        StatsResponse,
        EnqueueResponse,
        CreateTradeRequest,
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
    )),
    modifiers(&SecurityAddon),
    info(
        title = "Card Collection Price Tracker API",
        version = "0.1.0",
        description = "REST API for tracking Magic: The Gathering card prices",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT")
    ),
    tags(
        (name = "card", description = "Single card lookup, price history and sale offers (authentication required)"),
        (name = "collection", description = "Player's private collection (authentication required, no public catalog)"),
        (name = "search", description = "Public card search across all users' collections (authentication required)"),
        (name = "maintenance", description = "Maintenance operations (public)"),
        (name = "auth", description = "Authentication and user registration (authentication required)"),
        (name = "trades", description = "Trade requests between two collectors (authentication required)"),
        (name = "autocomplete", description = "Public username autocomplete (no authentication)"),
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
