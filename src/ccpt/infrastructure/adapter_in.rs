use crate::application::error::{AppError, InfraError};
use crate::domain::error::FunctionalError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub mod auth_extractor;
pub mod autocomplete;
pub mod card;
pub mod collection;
pub mod maintenance;
pub mod openapi;
pub mod search;
pub mod trade;
pub mod user;

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
                | FunctionalError::SelfTrade => StatusCode::BAD_REQUEST,
                FunctionalError::PriceNotFound
                | FunctionalError::CardNotFound
                | FunctionalError::TradeNotFound
                | FunctionalError::UserNotFound
                | FunctionalError::TradeCardNotFound => StatusCode::NOT_FOUND,
                FunctionalError::TradeAccessDenied => StatusCode::FORBIDDEN,
                FunctionalError::TradeNotModifiable
                | FunctionalError::TradeNotAcceptable
                | FunctionalError::TradeAlreadyAccepted
                | FunctionalError::TradeAlreadyFinalized
                | FunctionalError::TradeNotFullyAccepted
                | FunctionalError::TradeAlreadyConfirmed
                | FunctionalError::TradeNotCompleted
                | FunctionalError::TradeAlreadyRated
                | FunctionalError::TradeEmpty
                | FunctionalError::CardAlreadyReserved => StatusCode::CONFLICT,
            },
            AppError::Authentication(_) => StatusCode::UNAUTHORIZED,
            AppError::Infra(e) => match e {
                InfraError::CallError(_) => StatusCode::BAD_GATEWAY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
        };

        let body = Json(json!({
            "error": String::from(self)
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::AuthenticationError;

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
    fn card_already_reserved_returns_conflict_status() {
        let error = AppError::Functional(FunctionalError::CardAlreadyReserved);
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
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
