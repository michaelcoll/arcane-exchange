use super::dto::{
    CreateSealedProductRequest, PaginatedPurchasesResponse, PurchaseDetailResponse,
    PurchaseResponse, PurchasesParams, UpdatePurchaseRequest,
};
use crate::application::error::AppError;
use crate::domain::pagination::PageRequest;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use crate::infrastructure::adapter_in::parse_purchase_id;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;

/// Nested under `/purchases`: what a player bought to enrich their collection.
pub fn create_purchase_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(list_purchases).post(create_purchase))
        .route(
            "/{id}",
            get(get_purchase)
                .patch(update_purchase)
                .delete(delete_purchase),
        )
}

#[utoipa::path(
    get,
    path = "/purchases",
    params(
        ("page" = Option<u32>, Query, description = "Page number, 0-based (default 0). `page * page_size` must not exceed 2000"),
        ("page_size" = Option<u32>, Query, description = "Items per page, 1 to 100 (default 20)"),
    ),
    responses(
        (status = 200, description = "The Purchases of the authenticated user, by purchase date then creation date, most recent first", body = PaginatedPurchasesResponse),
        (status = 400, description = "Pagination out of bounds"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "purchases",
)]
pub(crate) async fn list_purchases(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<PurchasesParams>,
) -> Result<axum::Json<PaginatedPurchasesResponse>, AppError> {
    let result = state
        .list_purchases_use_case
        .list_purchases(
            user.id,
            PageRequest {
                page: params.page,
                page_size: params.page_size,
            },
        )
        .await?;

    Ok(axum::Json(PaginatedPurchasesResponse {
        items: result
            .items
            .into_iter()
            .map(PurchaseResponse::from)
            .collect(),
        total: result.total,
        page: result.pagination.page(),
        page_size: result.pagination.page_size(),
    }))
}

#[utoipa::path(
    get,
    path = "/purchases/{id}",
    params(("id" = uuid::Uuid, Path, description = "Purchase id")),
    responses(
        (status = 200, description = "The Purchase with its content: each line carries the current trend price of its card in its finish, when Cardmarket has one. Also the value of the content (lines without a trend price left out and counted) and its profitability against the amount, in cents and in percent. Nothing of this is stored", body = PurchaseDetailResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Purchase not found, or not owned by the caller; `code` is `purchase_not_found`"),
    ),
    security(("bearer_auth" = [])),
    tag = "purchases",
)]
pub(crate) async fn get_purchase(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::Json<PurchaseDetailResponse>, AppError> {
    let detail = state
        .get_purchase_use_case
        .get_purchase(user.id, parse_purchase_id(&id)?)
        .await?;

    Ok(axum::Json(detail.into()))
}

#[utoipa::path(
    post,
    path = "/purchases",
    request_body = CreateSealedProductRequest,
    responses(
        (status = 201, description = "Sealed product recorded", body = PurchaseResponse),
        (status = 400, description = "Amount not strictly positive; `code` is `invalid_purchase_price`"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Unknown set; `code` is `set_not_found`"),
    ),
    security(("bearer_auth" = [])),
    tag = "purchases",
)]
pub(crate) async fn create_purchase(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<CreateSealedProductRequest>,
) -> Result<(StatusCode, axum::Json<PurchaseResponse>), AppError> {
    let purchase = state
        .create_sealed_product_use_case
        .create_sealed_product(user.id, payload.into_domain()?)
        .await?;

    Ok((StatusCode::CREATED, axum::Json(purchase.into())))
}

#[utoipa::path(
    patch,
    path = "/purchases/{id}",
    params(("id" = uuid::Uuid, Path, description = "Purchase id")),
    request_body = UpdatePurchaseRequest,
    responses(
        (status = 200, description = "Purchase updated; the collection is untouched", body = PurchaseResponse),
        (status = 400, description = "Amount not strictly positive; `code` is `invalid_purchase_price`"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Purchase not found, or not owned by the caller (`purchase_not_found`); unknown set (`set_not_found`)"),
    ),
    security(("bearer_auth" = [])),
    tag = "purchases",
)]
pub(crate) async fn update_purchase(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(payload): axum::Json<UpdatePurchaseRequest>,
) -> Result<axum::Json<PurchaseResponse>, AppError> {
    let purchase = state
        .update_purchase_use_case
        .update_purchase(user.id, parse_purchase_id(&id)?, payload.into_domain()?)
        .await?;

    Ok(axum::Json(purchase.into()))
}

#[utoipa::path(
    delete,
    path = "/purchases/{id}",
    params(("id" = uuid::Uuid, Path, description = "Purchase id")),
    responses(
        (status = 204, description = "Purchase deleted with its content; no card leaves the collection"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Purchase not found, or not owned by the caller; `code` is `purchase_not_found`"),
    ),
    security(("bearer_auth" = [])),
    tag = "purchases",
)]
pub(crate) async fn delete_purchase(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .delete_purchase_use_case
        .delete_purchase(user.id, parse_purchase_id(&id)?)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
