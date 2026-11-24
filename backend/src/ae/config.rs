use std::env;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub port: u16,
    pub scryfall_rate_limit_tokens: u32,
    pub cardmarket_price_guides_url: String,
    pub edh_rec_base_url: String,
    pub scryfall_base_url: String,
    pub gatherer_base_url: String,
    pub clerk_frontend_api_url: String,
    /// Folder the card images are written to, shared with the frontend that serves them.
    pub card_images_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:password@localhost/postgres".to_string()),
            database_max_connections: parse_env_or("DATABASE_MAX_CONNECTIONS", 5),
            port: parse_env_or("BACKEND_PORT", 8080),
            scryfall_rate_limit_tokens: parse_env_or("SCRYFALL_RATE_LIMIT_TOKENS", 8),
            cardmarket_price_guides_url: env::var("CARDMARKET_PRICE_GUIDES_URL").unwrap_or_else(
                |_| {
                    "https://downloads.s3.cardmarket.com/productCatalog/priceGuide/price_guide_1.json"
                        .to_string()
                },
            ),
            edh_rec_base_url: env::var("EDHREC_BASE_URL")
                .unwrap_or_else(|_| "https://json.edhrec.com".to_string()),
            scryfall_base_url: env::var("SCRYFALL_BASE_URL")
                .unwrap_or_else(|_| "https://api.scryfall.com".to_string()),
            gatherer_base_url: env::var("GATHERER_BASE_URL")
                .unwrap_or_else(|_| "https://gatherer.wizards.com".to_string()),
            clerk_frontend_api_url: env::var("CLERK_FRONTEND_API_URL")
                .expect("CLERK_FRONTEND_API_URL must be set in environment variables"),
            card_images_dir: env::var("CARD_IMAGES_DIR")
                .expect("CARD_IMAGES_DIR must be set in environment variables")
                .into(),
        }
    }
}

fn parse_env_or<T>(key: &str, default: T) -> T
where
    T: FromStr,
    T::Err: std::fmt::Debug,
{
    match env::var(key) {
        Ok(value) => value
            .parse()
            .unwrap_or_else(|e| panic!("Invalid {key}: {value:?} ({e:?})")),
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // DATABASE_URL is deliberately left out of this list: dozens of `#[sqlx::test]` tests across
    // the crate read it in parallel to open their own connection, and removing it even briefly
    // makes them panic ("DATABASE_URL must be set"). The other keys below are only read by
    // `Config::from_env` / `create_infra`, so they carry no such risk.
    const ENV_VARS: &[&str] = &[
        "DATABASE_MAX_CONNECTIONS",
        "BACKEND_PORT",
        "SCRYFALL_RATE_LIMIT_TOKENS",
        "CARDMARKET_PRICE_GUIDES_URL",
        "EDHREC_BASE_URL",
        "SCRYFALL_BASE_URL",
        "GATHERER_BASE_URL",
        "CLERK_FRONTEND_API_URL",
        "CARD_IMAGES_DIR",
    ];

    // Environment variables are process-global state: each test starts from a clean one so it
    // does not depend on the order of the other tests in this module.
    fn reset_env() {
        for key in ENV_VARS {
            unsafe { env::remove_var(key) };
        }
    }

    fn set(key: &str, value: &str) {
        unsafe { env::set_var(key, value) };
    }

    // A single test holding every scenario: environment variables are process-global, so several
    // `#[test]` functions mutating the same keys in parallel would corrupt each other. Running
    // them sequentially in one test avoids the race.
    #[test]
    fn from_env_reads_and_validates_configuration() {
        reset_env();
        set("CLERK_FRONTEND_API_URL", "https://clerk.example.com");
        set("CARD_IMAGES_DIR", "/data/card-images");

        let config = Config::from_env();

        // This test never touches DATABASE_URL (see the comment on `ENV_VARS`), so check the same
        // fallback computation rather than a fixed value.
        let expected_database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/postgres".to_string());
        assert_eq!(config.database_url, expected_database_url);
        assert_eq!(config.database_max_connections, 5);
        assert_eq!(config.port, 8080);
        assert_eq!(config.scryfall_rate_limit_tokens, 8);
        assert_eq!(
            config.cardmarket_price_guides_url,
            "https://downloads.s3.cardmarket.com/productCatalog/priceGuide/price_guide_1.json"
        );
        assert_eq!(config.edh_rec_base_url, "https://json.edhrec.com");
        assert_eq!(config.scryfall_base_url, "https://api.scryfall.com");
        assert_eq!(config.gatherer_base_url, "https://gatherer.wizards.com");
        assert_eq!(config.clerk_frontend_api_url, "https://clerk.example.com");
        assert_eq!(config.card_images_dir, PathBuf::from("/data/card-images"));

        reset_env();
        set("DATABASE_MAX_CONNECTIONS", "42");
        set("BACKEND_PORT", "9090");
        set("SCRYFALL_RATE_LIMIT_TOKENS", "16");
        set("CLERK_FRONTEND_API_URL", "https://clerk.example.com");
        set("CARD_IMAGES_DIR", "/data/card-images");

        let config = Config::from_env();

        assert_eq!(config.database_max_connections, 42);
        assert_eq!(config.port, 9090);
        assert_eq!(config.scryfall_rate_limit_tokens, 16);

        reset_env();
        let result = std::panic::catch_unwind(Config::from_env);
        let message = *result.unwrap_err().downcast::<String>().unwrap();
        assert!(message.contains("CLERK_FRONTEND_API_URL must be set"));

        reset_env();
        set("CLERK_FRONTEND_API_URL", "https://clerk.example.com");
        let result = std::panic::catch_unwind(Config::from_env);
        let message = *result.unwrap_err().downcast::<String>().unwrap();
        assert!(message.contains("CARD_IMAGES_DIR must be set"));

        reset_env();
        set("CLERK_FRONTEND_API_URL", "https://clerk.example.com");
        set("CARD_IMAGES_DIR", "/data/card-images");
        set("BACKEND_PORT", "not-a-port");

        let result = std::panic::catch_unwind(Config::from_env);
        let message = *result.unwrap_err().downcast::<String>().unwrap();
        assert!(message.contains("Invalid BACKEND_PORT"));

        // Do not let BACKEND_PORT="not-a-port" leak into the other tests of the binary.
        reset_env();
    }
}
