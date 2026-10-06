use ae::config::Config;
use ae::infrastructure;
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::util::SubscriberInitExt;

fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
        .with(sentry::integrations::tracing::layer())
        .init();

    let _guard = sentry::init((
        "https://a9de037d8a32f68ba3a9b2fa13ab576f@o4511529669033984.ingest.de.sentry.io/4511529672507472",
        sentry::ClientOptions::new()
            .maybe_release(sentry::release_name!())
            .traces_sample_rate(1.0)
            // Capture user IPs and potentially sensitive headers when using HTTP server integrations
            // see https://docs.sentry.io/platforms/rust/data-management/data-collected for more info
            .send_default_pii(true),
    ));

    let config = Config::from_env();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let pool = PgPoolOptions::new()
                .max_connections(config.database_max_connections)
                .connect(config.database_url.as_str())
                .await
                .expect("Failed to create database connection pool !");

            info!("Database connection pool started.");

            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("Failed to migrate schema !");

            info!("Database schema migration done.");

            let infra = infrastructure::create_infra(pool, &config).await;

            let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
            let listener = TcpListener::bind(addr)
                .await
                .expect("Failed to bind to address !");

            info!("Listening on {}.", addr);

            axum::serve(listener, infra.into_make_service())
                .await
                .unwrap();
        });

    Ok(())
}
