use std::env;
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
                .unwrap_or_else(|_| "https://edhrec.com".to_string()),
            scryfall_base_url: env::var("SCRYFALL_BASE_URL")
                .unwrap_or_else(|_| "https://api.scryfall.com".to_string()),
            gatherer_base_url: env::var("GATHERER_BASE_URL")
                .unwrap_or_else(|_| "https://gatherer.wizards.com".to_string()),
            clerk_frontend_api_url: env::var("CLERK_FRONTEND_API_URL")
                .expect("CLERK_FRONTEND_API_URL must be set in environment variables"),
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

    // DATABASE_URL est délibérément absente de cette liste : des dizaines de tests `#[sqlx::test]`
    // dans tout le crate la lisent en parallèle pour ouvrir leur propre connexion. La faire
    // disparaître même brièvement ici les fait paniquer ("DATABASE_URL must be set"). Les autres
    // clés ci-dessous ne sont lues que par `Config::from_env` / `create_infra`, donc sans risque
    // similaire.
    const ENV_VARS: &[&str] = &[
        "DATABASE_MAX_CONNECTIONS",
        "BACKEND_PORT",
        "SCRYFALL_RATE_LIMIT_TOKENS",
        "CARDMARKET_PRICE_GUIDES_URL",
        "EDHREC_BASE_URL",
        "SCRYFALL_BASE_URL",
        "GATHERER_BASE_URL",
        "CLERK_FRONTEND_API_URL",
    ];

    // Les variables d'env sont un état global du process : chaque test repart d'un état propre
    // pour ne pas dépendre de l'ordre d'exécution des autres tests de ce module.
    fn reset_env() {
        for key in ENV_VARS {
            unsafe { env::remove_var(key) };
        }
    }

    fn set(key: &str, value: &str) {
        unsafe { env::set_var(key, value) };
    }

    // Un seul test regroupant tous les scénarios : les variables d'env sont un état global du
    // process, donc plusieurs tests `#[test]` mutant les mêmes clés en parallèle se corrompent
    // mutuellement. Les exécuter séquentiellement dans un seul test évite la course.
    #[test]
    fn from_env_reads_and_validates_configuration() {
        reset_env();
        set("CLERK_FRONTEND_API_URL", "https://clerk.example.com");

        let config = Config::from_env();

        // DATABASE_URL n'est jamais touchée par ce test (voir le commentaire sur `ENV_VARS`) ; on
        // vérifie donc le même calcul de fallback plutôt qu'une valeur fixe.
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
        assert_eq!(config.edh_rec_base_url, "https://edhrec.com");
        assert_eq!(config.scryfall_base_url, "https://api.scryfall.com");
        assert_eq!(config.gatherer_base_url, "https://gatherer.wizards.com");
        assert_eq!(config.clerk_frontend_api_url, "https://clerk.example.com");

        reset_env();
        set("DATABASE_MAX_CONNECTIONS", "42");
        set("BACKEND_PORT", "9090");
        set("SCRYFALL_RATE_LIMIT_TOKENS", "16");
        set("CLERK_FRONTEND_API_URL", "https://clerk.example.com");

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
        set("BACKEND_PORT", "not-a-port");

        let result = std::panic::catch_unwind(Config::from_env);
        let message = *result.unwrap_err().downcast::<String>().unwrap();
        assert!(message.contains("Invalid BACKEND_PORT"));

        // Ne pas laisser BACKEND_PORT="not-a-port" fuiter vers les autres tests du binaire.
        reset_env();
    }
}
