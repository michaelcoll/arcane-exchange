use super::dto::{
    AddTradeCardRequest, CreateTradeRequest, CreateTradeResponse, ListTradesParams,
    PaginatedTradesResponse, RateTradeRequest, RemoveTradeCardRequest, TradeDetailResponse,
};
use crate::application::error::AppError;
use crate::application::service::trade_service::TRADES_MAX_OFFSET;
use crate::domain::card::CardId;
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::Pagination;
use crate::domain::trade::{TradeId, TradeListQuery};
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum_extra::extract::Query;

pub fn create_trade_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", post(create_trade).get(list_trades))
        .route("/{trade_id}", get(get_trade))
        .route("/{trade_id}/cards", post(add_trade_card))
        .route("/{trade_id}/cards/remove", post(remove_trade_card))
        .route("/{trade_id}/accept", post(accept_trade))
        .route("/{trade_id}/abandon", post(abandon_trade))
        .route("/{trade_id}/confirm", post(confirm_trade))
        .route("/{trade_id}/rate", post(rate_trade))
}

#[utoipa::path(
    post,
    path = "/trades",
    request_body = CreateTradeRequest,
    responses(
        (status = 201, description = "Trade created (or the existing active trade with this user), without any card", body = CreateTradeResponse),
        (status = 400, description = "Invalid payload, or respondent is the caller"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Respondent username unknown"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn create_trade(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<CreateTradeRequest>,
) -> Result<(StatusCode, axum::Json<CreateTradeResponse>), AppError> {
    let id = state
        .create_trade_use_case
        .create_trade(user.id, payload.respondent_username)
        .await?;

    Ok((
        StatusCode::CREATED,
        axum::Json(CreateTradeResponse { id: id.to_string() }),
    ))
}

#[utoipa::path(
    post,
    path = "/trades/{trade_id}/cards",
    params(("trade_id" = uuid::Uuid, Path, description = "Trade id")),
    request_body = AddTradeCardRequest,
    responses(
        (status = 204, description = "Card added to the trade (quantity incremented if already present)"),
        (status = 400, description = "Invalid payload, or owner_username is not a party to this trade"),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Caller is not a party to this trade"),
        (status = 404, description = "Trade not found, owner username unknown, the caller doesn't own enough of the card, or the other party doesn't offer enough of it to trade (visibility/binders/rarity filters)"),
        (status = 409, description = "Trade cannot be modified in its current status, or the card is already reserved by another trade"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn add_trade_card(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(trade_id): Path<uuid::Uuid>,
    axum::Json(payload): axum::Json<AddTradeCardRequest>,
) -> Result<StatusCode, AppError> {
    let language_code = LanguageCode::try_new(&payload.language_code).map_err(AppError::from)?;
    let card_id = CardId::try_new(
        payload.set_code.as_str(),
        payload.collector_number,
        language_code,
        payload.foil,
    )
    .map_err(AppError::from)?;

    if payload.quantity == 0 {
        return Err(AppError::Functional(FunctionalError::WrongFormat(
            "quantity must be at least 1".to_string(),
        )));
    }

    state
        .add_trade_card_use_case
        .add_card(
            TradeId(trade_id),
            user.id,
            payload.owner_username,
            card_id,
            payload.quantity,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/trades/{trade_id}/cards/remove",
    params(("trade_id" = uuid::Uuid, Path, description = "Trade id")),
    request_body = RemoveTradeCardRequest,
    responses(
        (status = 204, description = "Card removed from the trade"),
        (status = 400, description = "Invalid payload, or owner_username is not a party to this trade"),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Caller is not a party to this trade"),
        (status = 404, description = "Trade not found, owner username unknown, or card not part of the trade"),
        (status = 409, description = "Trade cannot be modified in its current status"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn remove_trade_card(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(trade_id): Path<uuid::Uuid>,
    axum::Json(payload): axum::Json<RemoveTradeCardRequest>,
) -> Result<StatusCode, AppError> {
    let language_code = LanguageCode::try_new(&payload.language_code).map_err(AppError::from)?;
    let card_id = CardId::try_new(
        payload.set_code.as_str(),
        payload.collector_number,
        language_code,
        payload.foil,
    )
    .map_err(AppError::from)?;

    state
        .remove_trade_card_use_case
        .remove_card(TradeId(trade_id), user.id, payload.owner_username, card_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/trades/{trade_id}/accept",
    params(("trade_id" = uuid::Uuid, Path, description = "Trade id")),
    responses(
        (status = 204, description = "Trade accepted"),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Caller is not a party to this trade"),
        (status = 404, description = "Trade not found"),
        (status = 409, description = "Trade cannot be accepted in its current status, or already accepted by the caller"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn accept_trade(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(trade_id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    state
        .accept_trade_use_case
        .accept(TradeId(trade_id), user.id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/trades/{trade_id}/abandon",
    params(("trade_id" = uuid::Uuid, Path, description = "Trade id")),
    responses(
        (status = 204, description = "Trade abandoned"),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Caller is not a party to this trade"),
        (status = 404, description = "Trade not found"),
        (status = 409, description = "Trade is already finalized and cannot be abandoned"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn abandon_trade(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(trade_id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    state
        .abandon_trade_use_case
        .abandon(TradeId(trade_id), user.id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/trades/{trade_id}/confirm",
    params(("trade_id" = uuid::Uuid, Path, description = "Trade id")),
    responses(
        (status = 204, description = "Physical exchange confirmed"),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Caller is not a party to this trade"),
        (status = 404, description = "Trade not found"),
        (status = 409, description = "Trade must be fully accepted before it can be confirmed, or already confirmed by the caller"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn confirm_trade(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(trade_id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    state
        .confirm_trade_use_case
        .confirm(TradeId(trade_id), user.id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/trades/{trade_id}/rate",
    params(("trade_id" = uuid::Uuid, Path, description = "Trade id")),
    request_body = RateTradeRequest,
    responses(
        (status = 204, description = "Rating recorded"),
        (status = 400, description = "Rating missing or out of the 0-5 range"),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Caller is not a party to this trade"),
        (status = 404, description = "Trade not found"),
        (status = 409, description = "Trade must be completed before it can be rated, or already rated by the caller"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn rate_trade(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(trade_id): Path<uuid::Uuid>,
    axum::Json(payload): axum::Json<RateTradeRequest>,
) -> Result<StatusCode, AppError> {
    if payload.rating > 5 {
        return Err(AppError::Functional(FunctionalError::WrongFormat(
            "rating must be between 0 and 5".to_string(),
        )));
    }

    state
        .rate_trade_use_case
        .rate(TradeId(trade_id), user.id, payload.rating)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/trades/{trade_id}",
    params(("trade_id" = uuid::Uuid, Path, description = "Trade id")),
    responses(
        (status = 200, description = "Trade detail", body = TradeDetailResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Caller is not a party to this trade"),
        (status = 404, description = "Trade not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn get_trade(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(trade_id): Path<uuid::Uuid>,
) -> Result<axum::Json<TradeDetailResponse>, AppError> {
    let detail = state
        .get_trade_use_case
        .get_trade(TradeId(trade_id), user.id)
        .await?;

    Ok(axum::Json(TradeDetailResponse::from(detail)))
}

#[utoipa::path(
    get,
    path = "/trades",
    params(
        ("page" = Option<u32>, Query, description = "Page number, 0-based (default 0). `page * page_size` must not exceed 2000"),
        ("page_size" = Option<u32>, Query, description = "Items per page, 1 to 100 (default 20)"),
        ("status" = Option<Vec<super::dto::TradeStatusParam>>, Query, description = "Trade status, repeated for multiple values (e.g. status=PENDING&status=CLOSED)"),
    ),
    responses(
        (status = 200, description = "Paginated list of trades the caller is a party to", body = PaginatedTradesResponse),
        (status = 400, description = "Invalid status filter, or pagination out of bounds"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn list_trades(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<ListTradesParams>,
) -> Result<axum::Json<PaginatedTradesResponse>, AppError> {
    let pagination = Pagination::try_new(params.page, params.page_size, TRADES_MAX_OFFSET)?;

    let query = TradeListQuery {
        statuses: params.status.into_iter().map(Into::into).collect(),
        pagination,
    };

    let result = state
        .list_trades_use_case
        .list_trades(user.id, query)
        .await?;

    Ok(axum::Json(PaginatedTradesResponse {
        items: result
            .items
            .into_iter()
            .map(super::dto::TradeSummaryResponse::from)
            .collect(),
        total: result.total,
        page: result.pagination.page(),
        page_size: result.pagination.page_size(),
    }))
}
