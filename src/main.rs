use server_api_rt::{
    config::Config,
    create_app,
    logging::{init_logging, log_server_ready, log_shutdown, log_startup_info},
    services::{database::establish_connection, redis::RedisService},
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging system
    init_logging()?;

    tracing::info!("🚀 Starting Server API...");

    // Load configuration
    tracing::info!("📋 Loading configuration...");
    let config = match Config::from_env() {
        Ok(config) => {
            log_startup_info(&config);
            config
        }
        Err(e) => {
            tracing::error!("❌ Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    // Establish database connection
    tracing::info!("🔌 Connecting to database...");
    let db = match establish_connection(&config.database).await {
        Ok(db) => {
            tracing::info!("✅ Database connection established");
            db
        }
        Err(e) => {
            tracing::error!("❌ Failed to connect to database: {}", e);
            return Err(e.into());
        }
    };

    // Initialize Redis connection
    tracing::info!("🔗 Initializing Redis connection...");
    if let Err(e) = RedisService::init(&config.redis).await {
        tracing::error!("❌ Failed to connect to Redis: {}", e);
        return Err(e);
    }

    // Create application
    tracing::info!("🔧 Creating application...");
    let app = create_app(db);

    // Create socket address
    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));

    // Start server
    tracing::info!("🚀 Starting HTTP server...");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    log_server_ready(&addr);

    // Setup graceful shutdown
    let result = axum::serve(listener, app).await;

    log_shutdown();
    result.map_err(Into::into)
}
