use server_api_rt::{
    create_app,
    logging::{init_logging, log_server_ready, log_shutdown},
    services::{
        redis::RedisService, search::client::MeilisearchClient, utils::maintain_sentence_queue,
    },
    AppState,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging()?;

    let app_state = AppState::new().await?;

    tracing::info!("启动服务器 API...");

    tracing::info!("初始化 Redis 连接...");

    if let Err(e) = RedisService::init(app_state.config.redis.clone()).await {
        tracing::error!("Redis 连接失败: {}", e);
        return Err(e);
    }
    tracing::info!("启动预热一句话接口");
    maintain_sentence_queue().await;

    tracing::info!("启动搜索引擎...");
    if let Err(e) = MeilisearchClient::init(
        app_state.config.meilisearch.url.clone(),
        app_state.config.meilisearch.api_key.clone(),
    )
    .await
    {
        tracing::error!("Meilisearch 初始化失败: {}", e);
        return Err(e.into());
    }
    let client = MeilisearchClient::instance()?;

    let db = app_state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = client.sync_meilisearch_loop(&db, 60).await {
            tracing::error!("Meilisearch 同步失败: {}", e);
        }
    });

    tracing::info!("创建应用程序...");
    let app = create_app(app_state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], app_state.config.server.port));

    tracing::info!("启动 HTTP 服务器...");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    log_server_ready(&addr);

    let result = axum::serve(listener, app).await;

    log_shutdown();
    result.map_err(Into::into)
}
