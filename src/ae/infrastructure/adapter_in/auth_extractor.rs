use crate::domain::user::User;
use crate::infrastructure::AppState;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
};

#[derive(Debug)]
pub struct AuthenticatedUser(pub User);

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (StatusCode, String);

    #[allow(clippy::manual_async_fn)]
    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let auth_header = parts
                .headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    (
                        StatusCode::UNAUTHORIZED,
                        "Missing Authorization header".to_string(),
                    )
                })?;

            let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid Authorization header format".to_string(),
                )
            })?;

            let user = state
                .auth_service
                .validate_token(token)
                .await
                .map_err(|e| {
                    (
                        StatusCode::UNAUTHORIZED,
                        format!("Authentication failed: {}", e),
                    )
                })?;

            Ok(AuthenticatedUser(user))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::service::auth_service::{AuthService, MockAuthService};
    use crate::domain::user::UserId;
    use axum::http::Request;
    use std::sync::Arc;

    fn create_test_app_state_with_auth(auth_service: Arc<dyn AuthService>) -> AppState {
        AppState {
            auth_service,
            ..AppState::for_testing()
        }
    }

    #[tokio::test]
    async fn missing_authorization_header_returns_unauthorized() {
        let mock_auth = MockAuthService::new();

        let state = create_test_app_state_with_auth(Arc::new(mock_auth));
        let request = Request::builder().body(()).unwrap();
        let (mut parts, _) = request.into_parts();

        let result = AuthenticatedUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
        let (status, message) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(message, "Missing Authorization header");
    }

    #[tokio::test]
    async fn invalid_authorization_format_returns_unauthorized() {
        let mock_auth = MockAuthService::new();

        let state = create_test_app_state_with_auth(Arc::new(mock_auth));
        let request = Request::builder()
            .header(header::AUTHORIZATION, "Basic invalid")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let result = AuthenticatedUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
        let (status, message) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(message, "Invalid Authorization header format");
    }

    #[tokio::test]
    async fn valid_bearer_token_returns_user() {
        let expected_user = User::new("user_clerk123".to_string(), None, None, None);

        let mut mock_auth = MockAuthService::new();
        mock_auth
            .expect_validate_token()
            .with(mockall::predicate::eq("valid-token"))
            .returning(move |_| Ok(expected_user.clone()));

        let state = create_test_app_state_with_auth(Arc::new(mock_auth));
        let request = Request::builder()
            .header(header::AUTHORIZATION, "Bearer valid-token")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let result = AuthenticatedUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let AuthenticatedUser(user) = result.unwrap();
        assert_eq!(user.id, UserId::new("user_clerk123"));
        assert_eq!(user.name, None);
    }
}
