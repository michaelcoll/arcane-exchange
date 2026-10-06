use crate::application::error::{AppError, InfraError};
use crate::domain::card::CopyId;
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use ts_rs::TS;
use utoipa::ToSchema;

pub mod auth_extractor;
pub mod autocomplete;
pub mod card;
pub mod collection;
pub mod maintenance;
pub mod openapi;
pub mod search;
pub mod sets;
pub mod stats;
pub mod trade;
pub mod trade_settings;
pub mod user;

/// Builds the card copy a request designates by its raw `set_code` / `collector_number` /
/// `language_code` / `foil` fields. An unknown language code or an invalid collector number is a
/// functional error (400).
pub(crate) fn parse_copy_id(
    set_code: &str,
    collector_number: &str,
    language_code: &str,
    foil: bool,
) -> Result<CopyId, AppError> {
    let language_code = LanguageCode::try_new(language_code)?;
    Ok(CopyId::try_new(
        set_code,
        collector_number,
        language_code,
        foil,
    )?)
}

/// Body of every error response. `error` is a technical message, for diagnosis only; clients
/// translate `code` into what they show the user (ADR 0018).
#[derive(Serialize, Debug, TS, ToSchema)]
#[ts(export)]
pub struct ErrorResponse {
    /// Technical message, in English — never shown to the user as is.
    pub error: String,
    /// Stable snake_case identifier of the error (e.g. `binder_export`, `internal`).
    pub code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::Functional(e) => match e {
                FunctionalError::ParseError { .. }
                | FunctionalError::InvalidLanguageCode(_)
                | FunctionalError::InvalidSetCode(_)
                | FunctionalError::InvalidRarityCode(_)
                | FunctionalError::InvalidCollectorNumber(_)
                | FunctionalError::WrongFormat(_)
                | FunctionalError::EmptyFile
                | FunctionalError::MalformedCsv(_)
                | FunctionalError::BinderExport
                | FunctionalError::UnrecognizedFormat { .. }
                | FunctionalError::NoValidLine
                | FunctionalError::InvalidCardImportStatus(_)
                | FunctionalError::InvalidPageSize { .. }
                | FunctionalError::PaginationTooDeep { .. }
                | FunctionalError::AddedAtSortRequiresPlayerUsername
                | FunctionalError::SelfTrade => StatusCode::BAD_REQUEST,
                FunctionalError::PriceNotFound
                | FunctionalError::CardNotFound
                | FunctionalError::SetNotFound
                | FunctionalError::TradeNotFound
                | FunctionalError::UserNotFound
                | FunctionalError::TradeCardNotFound
                | FunctionalError::BinderNotFound
                | FunctionalError::ImportNotFound => StatusCode::NOT_FOUND,
                FunctionalError::TradeAccessDenied => StatusCode::FORBIDDEN,
                FunctionalError::TradeNotModifiable
                | FunctionalError::TradeNotAcceptable
                | FunctionalError::TradeAlreadyAccepted
                | FunctionalError::TradeAlreadyFinalized
                | FunctionalError::TradeNotFullyAccepted
                | FunctionalError::TradeAlreadyConfirmed
                | FunctionalError::TradeNotCompleted
                | FunctionalError::TradeAlreadyRated
                | FunctionalError::TradeConcurrentlyModified
                | FunctionalError::TradeEmpty
                | FunctionalError::CardAlreadyReserved
                | FunctionalError::ImportAlreadyRunning => StatusCode::CONFLICT,
            },
            AppError::Authentication(_) => StatusCode::UNAUTHORIZED,
            AppError::Infra(e) => match e {
                InfraError::CallError(_) => StatusCode::BAD_GATEWAY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
        };

        let body = Json(ErrorResponse {
            code: self.code().to_string(),
            error: String::from(self),
        });

        (status, body).into_response()
    }
}

/// Error responses built outside `AppError` — axum's extractor rejections, unknown routes — are
/// plain text. Rewrites them with the `ErrorResponse` body, so every error the API returns has
/// the documented shape: `invalid_request` for a client error, `internal` for a server one.
pub(crate) async fn with_error_body(response: Response) -> Response {
    let status = response.status();
    let is_json = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .is_some_and(|value| value.as_bytes().starts_with(b"application/json"));
    if !(status.is_client_error() || status.is_server_error()) || is_json {
        return response;
    }

    // These bodies are short diagnostics; past 64 KiB, the message is dropped rather than kept.
    let error = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    let code = if status.is_client_error() {
        "invalid_request"
    } else {
        "internal"
    };

    (
        status,
        Json(ErrorResponse {
            error,
            code: code.to_string(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::AuthenticationError;
    use serde_json::json;

    #[test]
    fn parse_copy_id_builds_the_copy() {
        let copy_id = parse_copy_id("FDN", "87", "FR", true).unwrap();

        assert_eq!(copy_id, CopyId::new("FDN", "87", LanguageCode::FR, true));
    }

    #[test]
    fn parse_copy_id_rejects_unknown_language_code() {
        let result = parse_copy_id("FDN", "87", "XX", false);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::InvalidLanguageCode(
                _
            )))
        ));
    }

    #[test]
    fn parse_copy_id_rejects_too_long_collector_number() {
        let result = parse_copy_id("FDN", "12345678901", "EN", false);

        assert!(matches!(
            result,
            Err(AppError::Functional(
                FunctionalError::InvalidCollectorNumber(_)
            ))
        ));
    }

    async fn body_of(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn error_body_carries_the_message_and_the_code() {
        let response = AppError::Functional(FunctionalError::BinderExport).into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_of(response).await,
            json!({
                "error": "expecting a collection export, got a binder export",
                "code": "binder_export"
            })
        );
    }

    #[tokio::test]
    async fn an_extractor_rejection_gets_the_error_body() {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct Params {
            page: u32,
        }
        let uri: axum::http::Uri = "/x?page=abc".parse().unwrap();
        let rejection = axum::extract::Query::<Params>::try_from_uri(&uri)
            .err()
            .unwrap()
            .into_response();

        let response = with_error_body(rejection).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_of(response).await;
        assert_eq!(body["code"], "invalid_request");
        assert!(body["error"].as_str().unwrap().contains("page"));
    }

    #[tokio::test]
    async fn a_bodiless_server_error_gets_the_internal_code() {
        let response = with_error_body(StatusCode::SERVICE_UNAVAILABLE.into_response()).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body_of(response).await["code"], "internal");
    }

    #[tokio::test]
    async fn an_app_error_body_is_left_as_is() {
        let response =
            with_error_body(AppError::Functional(FunctionalError::CardNotFound).into_response())
                .await;

        assert_eq!(body_of(response).await["code"], "card_not_found");
    }

    #[tokio::test]
    async fn a_success_is_left_as_is() {
        let response = with_error_body((StatusCode::OK, "plain").into_response()).await;

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&bytes[..], b"plain");
    }

    #[tokio::test]
    async fn infra_error_body_carries_the_internal_code() {
        let response =
            AppError::Infra(InfraError::RepositoryError("db down".to_string())).into_response();

        assert_eq!(body_of(response).await["code"], "internal");
    }

    #[test]
    fn import_format_errors_return_bad_request_status() {
        for error in [
            FunctionalError::EmptyFile,
            FunctionalError::MalformedCsv("bad quote".to_string()),
            FunctionalError::BinderExport,
            FunctionalError::UnrecognizedFormat {
                missing: vec!["Proxy".to_string()],
                unexpected: vec![],
            },
            FunctionalError::NoValidLine,
        ] {
            let response = AppError::Functional(error).into_response();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[test]
    fn parse_error_returns_bad_request_status() {
        let error = AppError::Functional(FunctionalError::ParseError {
            line: 5,
            field: "quantity",
            value: "invalid".to_string(),
        });
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn parse_error_with_different_line_number_returns_bad_request() {
        let error = AppError::Functional(FunctionalError::ParseError {
            line: 42,
            field: "foil",
            value: "maybe".to_string(),
        });
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn wrong_format_returns_bad_request_status() {
        let error = AppError::Functional(FunctionalError::WrongFormat(
            "CSV header missing".to_string(),
        ));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn wrong_format_with_empty_message_returns_bad_request() {
        let error = AppError::Functional(FunctionalError::WrongFormat(String::new()));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn price_not_found_returns_not_found_status() {
        let error = AppError::Functional(FunctionalError::PriceNotFound);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn card_not_found_returns_not_found_status() {
        let error = AppError::Functional(FunctionalError::CardNotFound);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn self_trade_returns_bad_request_status() {
        let error = AppError::Functional(FunctionalError::SelfTrade);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn trade_not_modifiable_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeNotModifiable);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_not_found_returns_not_found_status() {
        let error = AppError::Functional(FunctionalError::TradeNotFound);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn trade_access_denied_returns_forbidden_status() {
        let error = AppError::Functional(FunctionalError::TradeAccessDenied);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn trade_not_acceptable_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeNotAcceptable);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_already_accepted_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeAlreadyAccepted);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_already_finalized_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeAlreadyFinalized);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_not_fully_accepted_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeNotFullyAccepted);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_already_confirmed_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeAlreadyConfirmed);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_not_completed_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeNotCompleted);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_already_rated_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeAlreadyRated);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn user_not_found_returns_not_found_status() {
        let error = AppError::Functional(FunctionalError::UserNotFound);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn trade_empty_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::TradeEmpty);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn trade_card_not_found_returns_not_found_status() {
        let error = AppError::Functional(FunctionalError::TradeCardNotFound);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn binder_not_found_returns_not_found_status() {
        let error = AppError::Functional(FunctionalError::BinderNotFound);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn card_already_reserved_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::CardAlreadyReserved);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn invalid_page_size_returns_bad_request_status() {
        let error = AppError::Functional(FunctionalError::InvalidPageSize {
            requested: 0,
            max: 100,
        });
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn pagination_too_deep_returns_bad_request_status() {
        let error = AppError::Functional(FunctionalError::PaginationTooDeep {
            requested_offset: 500,
            max: 100,
        });
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn added_at_sort_requires_player_username_returns_bad_request_status() {
        let error = AppError::Functional(FunctionalError::AddedAtSortRequiresPlayerUsername);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn call_error_returns_bad_gateway_status() {
        let error = AppError::Infra(InfraError::CallError("External API timeout".to_string()));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn call_error_with_network_failure_returns_bad_gateway() {
        let error = AppError::Infra(InfraError::CallError("Connection refused".to_string()));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn repository_error_returns_internal_server_error_status() {
        let error = AppError::Infra(InfraError::RepositoryError(
            "Database connection lost".to_string(),
        ));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn queue_error_returns_internal_server_error_status() {
        let error = AppError::Infra(InfraError::QueueError("Queue overflow".to_string()));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn authentication_error_returns_unauthorized_status() {
        let error = AppError::Authentication(AuthenticationError::InvalidToken(
            "Invalid credentials".to_string(),
        ));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
