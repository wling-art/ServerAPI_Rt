use server_api_rt::{
    create_app,
    logging::{init_logging, log_server_ready, log_shutdown},
    services::{redis::RedisService, utils::{maintain_sentence_queue, preload_sentence_queue}},
    AppState,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging()?;

    let app_state = AppState::new().await?;

    tracing::info!("🚀 启动服务器 API...");

    tracing::info!("🔗 初始化 Redis 连接...");

    if let Err(e) = RedisService::init(app_state.config.redis.clone()).await {
        tracing::error!("❌ Redis 连接失败: {}", e);
        return Err(e);
    }
    tracing::info!("🔗 预热一句话接口");
    // 预加载句子队列
    preload_sentence_queue().await;

    // 启动后台维护任务
    maintain_sentence_queue().await;
    tracing::info!("🔧 创建应用程序...");
    let app = create_app(app_state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], app_state.config.server.port));

    tracing::info!("🚀 启动 HTTP 服务器...");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    log_server_ready(&addr);

    let result = axum::serve(listener, app).await;

    log_shutdown();
    result.map_err(Into::into)
}
