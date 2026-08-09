use super::dto::{
    CreateTradeRequest, ListTradesParams, PaginatedTradesResponse, RateTradeRequest,
    TradeDetailResponse,
};
use crate::application::error::AppError;
use crate::domain::card::CardId;
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::trade::{TradeId, TradeListQuery};
use crate::domain::user::UserId;
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
        (status = 201, description = "Trade created, or card added to (merged into) the active trade with this user"),
        (status = 400, description = "Invalid payload or respondent is the initiator"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Card not found, not owned by respondent, or respondent unknown"),
        (status = 409, description = "The active trade with this user is already fully accepted and can no longer be modified"),
    ),
    security(("bearer_auth" = [])),
    tag = "trades",
)]
pub(crate) async fn create_trade(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<CreateTradeRequest>,
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
        .create_trade_use_case
        .create_trade(
            user.id,
            UserId::new(payload.respondent_user_id),
            card_id,
            payload.quantity,
        )
        .await?;

    Ok(StatusCode::CREATED)
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
        ("page" = Option<u32>, Query, description = "Page number (starts at 0)"),
        ("page_size" = Option<u32>, Query, description = "Items per page (max 100)"),
        ("status" = Option<Vec<super::dto::TradeStatusParam>>, Query, description = "Trade status, repeated for multiple values (e.g. status=PENDING&status=CLOSED)"),
    ),
    responses(
        (status = 200, description = "Paginated list of trades the caller is a party to", body = PaginatedTradesResponse),
        (status = 400, description = "Invalid status filter"),
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
    let page_size = params.page_size.clamp(1, state.max_page_size);
    let page = params.page.min(state.max_page_number);

    let query = TradeListQuery {
        statuses: params.status.into_iter().map(Into::into).collect(),
        page,
        page_size,
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
        page: result.page,
        page_size: result.page_size,
    }))
}
