use crate::domain::error::FunctionalError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfraError {
    RepositoryError(String),
    CallError(String),
    QueueError(String),
}

impl From<InfraError> for String {
    fn from(val: InfraError) -> String {
        match val {
            InfraError::RepositoryError(msg) => msg,
            InfraError::CallError(msg) => msg,
            InfraError::QueueError(msg) => msg,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticationError {
    InvalidToken(String),
}

impl From<AuthenticationError> for String {
    fn from(val: AuthenticationError) -> String {
        match val {
            AuthenticationError::InvalidToken(msg) => format!("Authentication error: {}", msg),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppError {
    Functional(FunctionalError),
    Authentication(AuthenticationError),
    Infra(InfraError),
}

impl From<AppError> for String {
    fn from(val: AppError) -> String {
        match val {
            AppError::Functional(e) => e.into(),
            AppError::Authentication(e) => e.into(),
            AppError::Infra(e) => e.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self.clone()))
    }
}

impl std::error::Error for AppError {}

impl From<FunctionalError> for AppError {
    fn from(error: FunctionalError) -> Self {
        AppError::Functional(error)
    }
}

impl From<AuthenticationError> for AppError {
    fn from(error: AuthenticationError) -> Self {
        AppError::Authentication(error)
    }
}

impl From<InfraError> for AppError {
    fn from(error: InfraError) -> Self {
        AppError::Infra(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_to_string_for_parse_error() {
        let error = AppError::Functional(FunctionalError::ParseError {
            line: 1,
            field: "quantity",
            value: "abc".to_string(),
        });
        let error_message: String = error.into();
        assert_eq!(
            error_message,
            "Line 1: invalid quantity 'abc' (must be a valid value)"
        );
    }

    #[test]
    fn app_error_to_string_for_wrong_format() {
        let error =
            AppError::Functional(FunctionalError::WrongFormat("Invalid format".to_string()));
        let error_message: String = error.into();
        assert_eq!(error_message, "Invalid format");
    }

    #[test]
    fn app_error_to_string_for_repository_error() {
        let error = AppError::Infra(InfraError::RepositoryError("Repository error".to_string()));
        let error_message: String = error.into();
        assert_eq!(error_message, "Repository error");
    }

    #[test]
    fn app_error_to_string_for_price_not_found() {
        let error = AppError::Functional(FunctionalError::PriceNotFound);
        let error_message: String = error.into();
        assert_eq!(error_message, "Price not found");
    }

    #[test]
    fn app_error_to_string_for_card_not_found() {
        let error = AppError::Functional(FunctionalError::CardNotFound);
        let error_message: String = error.into();
        assert_eq!(error_message, "Card not found");
    }

    #[test]
    fn app_error_to_string_for_call_error() {
        let error = AppError::Infra(InfraError::CallError("Call failed".to_string()));
        let error_message: String = error.into();
        assert_eq!(error_message, "Call failed");
    }

    #[test]
    fn app_error_to_string_for_queue_error() {
        let error = AppError::Infra(InfraError::QueueError("Queue operation failed".to_string()));
        let error_message: String = error.into();
        assert_eq!(error_message, "Queue operation failed");
    }

    #[test]
    fn app_error_to_string_for_authentication_error() {
        let error = AppError::Authentication(AuthenticationError::InvalidToken(
            "Invalid credentials".to_string(),
        ));
        let error_message: String = error.into();
        assert_eq!(error_message, "Authentication error: Invalid credentials");
    }

    #[test]
    fn app_error_to_string_for_self_trade() {
        let error = AppError::Functional(FunctionalError::SelfTrade);
        let error_message: String = error.into();
        assert_eq!(error_message, "Cannot request your own card");
    }

    #[test]
    fn app_error_to_string_for_trade_not_modifiable() {
        let error = AppError::Functional(FunctionalError::TradeNotModifiable);
        let error_message: String = error.into();
        assert_eq!(
            error_message,
            "This trade has already been fully accepted and can no longer be modified"
        );
    }
}
