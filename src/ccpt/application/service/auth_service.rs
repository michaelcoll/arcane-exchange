use crate::application::error::{AppError, AuthenticationError, InfraError};
use crate::domain::user::User;
use async_trait::async_trait;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct ClerkClaims {
    sub: String,
    username: Option<String>,
    image_url: Option<String>,
    exp: usize,
    azp: Option<String>,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AuthService: Send + Sync {
    async fn validate_token(&self, token: &str) -> Result<User, AppError>;
}

pub struct ClerkAuthService {
    jwks: JwkSet,
    frontend_api_url: String,
}

impl ClerkAuthService {
    /// Creates a new ClerkAuthService instance
    ///
    /// # Arguments
    /// * `frontend_api_url` - The Clerk Frontend API URL (e.g. `https://musical-pup-67.clerk.accounts.dev`)
    /// * `jwks_url` - The URL to fetch the JWKS from. If None, defaults to
    ///   `{frontend_api_url}/.well-known/jwks.json`
    pub async fn new(frontend_api_url: String, jwks_url: Option<&str>) -> Result<Self, AppError> {
        let url = if let Some(url) = jwks_url {
            url.to_string()
        } else {
            format!(
                "{}/.well-known/jwks.json",
                frontend_api_url.trim_end_matches('/')
            )
        };

        info!("Fetching JWKS from URL: {}", url);

        let jwks: JwkSet = reqwest::get(&url)
            .await
            .map_err(|e| InfraError::CallError(format!("Failed to fetch Clerk JWKS: {}", e)))?
            .json()
            .await
            .map_err(|e| InfraError::CallError(format!("Failed to parse Clerk JWKS: {}", e)))?;

        Ok(Self {
            jwks,
            frontend_api_url,
        })
    }

    /// Finds a JWK (JSON Web Key) by its key ID in the JWKS set
    fn find_jwk(&self, kid: &str) -> Option<&jsonwebtoken::jwk::Jwk> {
        self.jwks
            .keys
            .iter()
            .find(|k| k.common.key_id.as_deref() == Some(kid))
    }
}

#[async_trait]
impl AuthService for ClerkAuthService {
    async fn validate_token(&self, token: &str) -> Result<User, AppError> {
        let header = decode_header(token).map_err(|e| {
            AuthenticationError::InvalidToken(format!("Invalid token header: {}", e))
        })?;

        let kid = header
            .kid
            .ok_or_else(|| AuthenticationError::InvalidToken("Token missing kid".to_string()))?;

        let jwk = self
            .find_jwk(&kid)
            .ok_or_else(|| AuthenticationError::InvalidToken("Unknown key ID".to_string()))?;

        let decoding_key = DecodingKey::from_jwk(jwk)
            .map_err(|e| AuthenticationError::InvalidToken(format!("Invalid JWK: {}", e)))?;

        // Use the algorithm specified in the token header (RS256 or ES256 depending on Clerk config)
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&self.frontend_api_url]);
        // Clerk uses `azp` rather than `aud` for session tokens — disable audience validation
        validation.validate_aud = false;

        let token_data = decode::<ClerkClaims>(token, &decoding_key, &validation).map_err(|e| {
            AuthenticationError::InvalidToken(format!("Token validation failed: {}", e))
        })?;

        Ok(User::new(
            token_data.claims.sub,
            None, // Clerk JWT does not include name
            token_data.claims.username,
            token_data.claims.image_url,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn clerk_claims_deserialization() {
        let json = r#"{
            "sub": "user_clerk123",
            "exp": 1234567890
        }"#;

        let claims: ClerkClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user_clerk123");
    }

    #[test]
    fn clerk_claims_deserialization_with_username() {
        let json = r#"{
            "sub": "user_clerk123",
            "username": "clerkuser",
            "exp": 1234567890
        }"#;

        let claims: ClerkClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.username, Some("clerkuser".to_string()));
    }

    #[test]
    fn clerk_claims_deserialization_without_username_stays_valid() {
        let json = r#"{
            "sub": "user_clerk123",
            "exp": 1234567890
        }"#;

        let claims: ClerkClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.username, None);
    }

    #[test]
    fn clerk_claims_deserialization_with_image_url() {
        let json = r#"{
            "sub": "user_clerk123",
            "username": "clerkuser",
            "image_url": "https://img.clerk.com/avatar.png",
            "exp": 1234567890
        }"#;

        let claims: ClerkClaims = serde_json::from_str(json).unwrap();
        assert_eq!(
            claims.image_url,
            Some("https://img.clerk.com/avatar.png".to_string())
        );
    }

    #[test]
    fn clerk_claims_deserialization_without_image_url_stays_valid() {
        let json = r#"{
            "sub": "user_clerk123",
            "username": "clerkuser",
            "exp": 1234567890
        }"#;

        let claims: ClerkClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.image_url, None);
    }

    #[test]
    fn creates_service_with_frontend_api_url() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "test-key-id",
                    "n": "test-n-value",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                }
            ]
        }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        assert_eq!(
            service.frontend_api_url,
            "https://musical-pup-67.clerk.accounts.dev"
        );
        assert_eq!(service.jwks.keys.len(), 1);
    }

    #[test]
    fn stores_empty_jwks_set() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        assert!(service.jwks.keys.is_empty());
    }

    #[test]
    fn finds_jwk_by_kid_when_present() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "key-1",
                    "n": "test-n",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                },
                {
                    "kty": "RSA",
                    "kid": "key-2",
                    "n": "test-n-2",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                }
            ]
        }"#;

        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let result = service.find_jwk("key-1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().common.key_id.as_deref(), Some("key-1"));
    }

    #[test]
    fn returns_none_when_jwk_kid_not_found() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "key-1",
                    "n": "test-n",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                }
            ]
        }"#;

        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let result = service.find_jwk("non-existent-key");
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_jwks_empty() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let result = service.find_jwk("any-key");
        assert!(result.is_none());
    }

    // Tests for ClerkAuthService::new
    #[tokio::test]
    async fn new_successfully_creates_service_with_valid_jwks() {
        let server = MockServer::start().await;
        let jwks_response = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "test-key",
                    "n": "test-n",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                }
            ]
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_response))
            .mount(&server)
            .await;

        let service = ClerkAuthService::new(
            "https://musical-pup-67.clerk.accounts.dev".to_string(),
            Some(&server.uri()),
        )
        .await
        .unwrap();

        assert_eq!(
            service.frontend_api_url,
            "https://musical-pup-67.clerk.accounts.dev"
        );
        assert_eq!(service.jwks.keys.len(), 1);
        assert!(service.find_jwk("test-key").is_some());
    }

    #[tokio::test]
    async fn new_builds_jwks_url_from_frontend_api_url() {
        let server = MockServer::start().await;
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "key-1",
                    "n": "test-n-1",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                },
                {
                    "kty": "RSA",
                    "kid": "key-2",
                    "n": "test-n-2",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                }
            ]
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&server)
            .await;

        let service = ClerkAuthService::new(
            "https://musical-pup-67.clerk.accounts.dev".to_string(),
            Some(&server.uri()),
        )
        .await
        .unwrap();

        assert_eq!(
            service.frontend_api_url,
            "https://musical-pup-67.clerk.accounts.dev"
        );
        assert_eq!(service.jwks.keys.len(), 2);
        assert!(service.find_jwk("key-1").is_some());
        assert!(service.find_jwk("key-2").is_some());
    }

    // Tests for JWK decoding errors
    #[tokio::test]
    async fn rejects_token_with_invalid_jwk_format() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "test-key",
                    "n": "incomplete_n_value",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                }
            ]
        }"#;

        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let token_with_invalid_jwk = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3Qta2V5IiwidHlwIjoiSldUIn0.eyJzdWIiOiIxMjMiLCJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20iLCJpc3MiOiJodHRwczovL3Rlc3QuY2xlcmsuYWNjb3VudHMuZGV2IiwiZXhwIjo5OTk5OTk5OTk5fQ.signature";

        let result = service.validate_token(token_with_invalid_jwk).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(msg.contains("Invalid JWK") || msg.contains("Token validation failed"));
            }
            _ => panic!("Expected AuthenticationError for invalid JWK format"),
        }
    }

    #[tokio::test]
    async fn rejects_token_when_jwk_conversion_fails() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "EC",
                    "kid": "test-key",
                    "crv": "P-256",
                    "x": "xnK82U4TiifQvt1fYd8E9kI-X4pALyVX3jIaHJKz6vc",
                    "y": "3pj3S3Y21R9fDgYX-xSB_WLcXl8hCvA0-xR9oEQOXEk",
                    "alg": "ES256",
                    "use": "sig"
                }
            ]
        }"#;

        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let token = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3Qta2V5IiwidHlwIjoiSldUIn0.eyJzdWIiOiIxMjMiLCJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20iLCJpc3MiOiJodHRwczovL3Rlc3QuY2xlcmsuYWNjb3VudHMuZGV2IiwiZXhwIjo5OTk5OTk5OTk5fQ.signature";

        let result = service.validate_token(token).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(
                    msg.contains("Invalid JWK")
                        || msg.contains("Token validation failed")
                        || msg.contains("Couldn't convert")
                );
            }
            _ => panic!("Expected AuthenticationError for JWK conversion failure"),
        }
    }

    // Tests for token decoding errors
    #[tokio::test]
    async fn rejects_token_with_missing_kid_in_header() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let token_without_kid =
            "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature";

        let result = service.validate_token(token_without_kid).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(msg.contains("Token missing kid"));
            }
            _ => panic!("Expected AuthenticationError with missing kid message"),
        }
    }

    #[tokio::test]
    async fn rejects_token_with_unknown_kid() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let token_with_unknown_kid = "eyJhbGciOiJSUzI1NiIsImtpZCI6InVua25vd24ta2V5IiwidHlwIjoiSldUIn0.eyJzdWIiOiIxMjMiLCJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20iLCJpc3MiOiJodHRwczovL211c2ljYWwtcHVwLTY3LmNsZXJrLmFjY291bnRzLmRldiIsImV4cCI6OTk5OTk5OTk5OX0.signature";

        let result = service.validate_token(token_with_unknown_kid).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(msg.contains("Unknown key ID"));
            }
            _ => panic!("Expected AuthenticationError with unknown key ID message"),
        }
    }

    /// Clerk emits a non-standard `oiat` (original issued at) header as a JSON **integer**.
    /// jsonwebtoken <= 10.x typed `Header.extras` as `HashMap<String, String>`, so such a header
    /// failed to deserialize and every Clerk token was rejected at `decode_header` — the reason a
    /// patched fork of the crate was used. Since 11.0.0 `extras` holds `serde_json::Value`, which
    /// accepts it. Reaching "Unknown key ID" (and not "Invalid token header") proves the header was
    /// parsed and the `kid` read, so this test guards against reintroducing that regression.
    #[tokio::test]
    async fn accepts_token_header_with_integer_oiat_field() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        // Header decodes to {"alg":"RS256","kid":"ins_2abcTESTkey","typ":"JWT","oiat":1778751753}
        let token_with_oiat_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6Imluc18yYWJjVEVTVGtleSIsInR5cCI6IkpXVCIsIm9pYXQiOjE3Nzg3NTE3NTN9.eyJzdWIiOiJ1c2VyX2NsZXJrMTIzIiwiZXhwIjo5OTk5OTk5OTk5LCJpc3MiOiJodHRwczovL211c2ljYWwtcHVwLTY3LmNsZXJrLmFjY291bnRzLmRldiJ9.signature";

        let result = service.validate_token(token_with_oiat_header).await;

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(
                    msg.contains("Unknown key ID"),
                    "header with an integer `oiat` must parse; got: {}",
                    msg
                );
            }
            _ => panic!("Expected AuthenticationError with unknown key ID message"),
        }
    }

    #[tokio::test]
    async fn rejects_malformed_token() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let result = service.validate_token("not-a-valid-token").await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(msg.contains("Invalid token header"));
            }
            _ => panic!("Expected AuthenticationError for invalid header"),
        }
    }

    #[tokio::test]
    async fn rejects_token_with_invalid_base64() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let result = service
            .validate_token("invalid!!!.payload!!!.signature!!!")
            .await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(msg.contains("Invalid token header"));
            }
            _ => panic!("Expected AuthenticationError for invalid base64"),
        }
    }

    #[tokio::test]
    async fn rejects_token_with_wrong_algorithm() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "test-key",
                    "n": "test-n",
                    "e": "AQAB",
                    "alg": "RS256",
                    "use": "sig"
                }
            ]
        }"#;

        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let token_wrong_algo = "eyJhbGciOiJIUzI1NiIsImtpZCI6InRlc3Qta2V5IiwidHlwIjoiSldUIn0.eyJzdWIiOiIxMjMiLCJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20iLCJpc3MiOiJodHRwczovL211c2ljYWwtcHVwLTY3LmNsZXJrLmFjY291bnRzLmRldiIsImV4cCI6OTk5OTk5OTk5OX0.signature";

        let result = service.validate_token(token_wrong_algo).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(_msg))) => {
                // Error is expected for wrong algorithm
            }
            _ => panic!("Expected AuthenticationError for wrong algorithm"),
        }
    }

    #[tokio::test]
    async fn rejects_empty_token() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let result = service.validate_token("").await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(msg.contains("Invalid token header"));
            }
            _ => panic!("Expected AuthenticationError for empty token"),
        }
    }

    #[tokio::test]
    async fn rejects_token_with_incomplete_jwt_parts() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let incomplete_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3Qta2V5In0.eyJzdWIiOiIxMjMifQ";

        let result = service.validate_token(incomplete_token).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(msg))) => {
                assert!(msg.contains("Invalid token header"));
            }
            _ => panic!("Expected AuthenticationError for incomplete JWT"),
        }
    }

    #[tokio::test]
    async fn rejects_token_with_corrupted_payload() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let corrupted_token =
            "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3Qta2V5In0.!!!invalid_base64!!!.signature";

        let result = service.validate_token(corrupted_token).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(_msg))) => {
                // Error is expected for corrupted payload
            }
            _ => panic!("Expected AuthenticationError for corrupted payload"),
        }
    }

    #[tokio::test]
    async fn new_handles_multiple_keys_in_jwks() {
        let server = MockServer::start().await;
        let jwks_json = r#"{
            "keys": [
                { "kty": "RSA", "kid": "rsa-key-1", "n": "test-n-1", "e": "AQAB" },
                { "kty": "RSA", "kid": "rsa-key-2", "n": "test-n-2", "e": "AQAB" },
                { "kty": "RSA", "kid": "rsa-key-3", "n": "test-n-3", "e": "AQAB" }
            ]
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&server)
            .await;

        let service = ClerkAuthService::new(
            "https://musical-pup-67.clerk.accounts.dev".to_string(),
            Some(&server.uri()),
        )
        .await
        .unwrap();

        assert_eq!(service.jwks.keys.len(), 3);
        assert!(service.find_jwk("rsa-key-1").is_some());
        assert!(service.find_jwk("rsa-key-2").is_some());
        assert!(service.find_jwk("rsa-key-3").is_some());
        assert!(service.find_jwk("rsa-key-4").is_none());
    }

    #[tokio::test]
    async fn new_returns_error_when_response_is_not_json() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let result = ClerkAuthService::new(
            "https://musical-pup-67.clerk.accounts.dev".to_string(),
            Some(&server.uri()),
        )
        .await;

        assert!(result.is_err());
        match result {
            Err(AppError::Infra(InfraError::CallError(msg))) => {
                assert!(msg.contains("Failed to parse Clerk JWKS"));
            }
            _ => panic!("Expected CallError when response is not JSON"),
        }
    }

    #[tokio::test]
    async fn new_returns_error_on_invalid_json() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let result = ClerkAuthService::new(
            "https://musical-pup-67.clerk.accounts.dev".to_string(),
            Some(&server.uri()),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn new_returns_error_on_http_404() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let result = ClerkAuthService::new(
            "https://musical-pup-67.clerk.accounts.dev".to_string(),
            Some(&server.uri()),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn new_successfully_parses_jwks_with_all_fields() {
        let server = MockServer::start().await;
        let complete_jwks = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "complete-key",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "n_value_here",
                    "e": "AQAB"
                }
            ]
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(complete_jwks))
            .mount(&server)
            .await;

        let service = ClerkAuthService::new(
            "https://musical-pup-67.clerk.accounts.dev".to_string(),
            Some(&server.uri()),
        )
        .await
        .unwrap();

        assert_eq!(
            service.frontend_api_url,
            "https://musical-pup-67.clerk.accounts.dev"
        );
        assert_eq!(service.jwks.keys.len(), 1);
        assert!(service.find_jwk("complete-key").is_some());
    }

    #[tokio::test]
    async fn rejects_token_with_special_characters_in_payload() {
        let jwks_json = r#"{ "keys": [] }"#;
        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let token_with_special_chars =
            "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3Qta2V5In0.😀😀😀😀😀.signature";

        let result = service.validate_token(token_with_special_chars).await;
        assert!(result.is_err());

        match result {
            Err(AppError::Authentication(AuthenticationError::InvalidToken(_msg))) => {
                // Error is expected for special characters
            }
            _ => panic!("Expected AuthenticationError for special characters"),
        }
    }

    #[test]
    fn finds_correct_jwk_among_many_keys() {
        let jwks_json = r#"{
            "keys": [
                { "kty": "RSA", "kid": "wrong-key-1", "n": "test-n", "e": "AQAB" },
                { "kty": "RSA", "kid": "correct-key", "n": "correct-n", "e": "AQAB" },
                { "kty": "RSA", "kid": "wrong-key-2", "n": "test-n", "e": "AQAB" }
            ]
        }"#;

        let jwks: JwkSet = serde_json::from_str(jwks_json).unwrap();
        let service = ClerkAuthService {
            jwks,
            frontend_api_url: "https://musical-pup-67.clerk.accounts.dev".to_string(),
        };

        let result = service.find_jwk("correct-key");
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().common.key_id.as_deref(),
            Some("correct-key")
        );
    }
}
