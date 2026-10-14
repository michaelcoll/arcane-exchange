use crate::application::error::AppError;
use crate::domain::user::User;
use async_trait::async_trait;

/// Validates a bearer token and returns the user it authenticates.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TokenValidator: Send + Sync {
    async fn validate_token(&self, token: &str) -> Result<User, AppError>;
}
