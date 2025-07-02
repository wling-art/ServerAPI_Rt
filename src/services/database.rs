use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection as SeaOrmDatabaseConnection,
    DbErr,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::config::DatabaseConfig;

pub type DatabaseConnection = Arc<SeaOrmDatabaseConnection>;

pub async fn establish_connection(config: &DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(&config.url);

    // 设置连接池参数
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_secs(config.connect_timeout))
        .acquire_timeout(Duration::from_secs(config.acquire_timeout))
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .max_lifetime(Duration::from_secs(28800)) // 8 hours
        .sqlx_logging(false); // 暂时禁用 SQL 日志以避免类型错误

    info!(
        "🔗 Configuring database connection pool: min={}, max={}",
        config.min_connections, config.max_connections
    );

    let db = Database::connect(opt).await?;
    let connection = Arc::new(db);

    // 预热连接池
    if let Err(e) = warm_up_connection_pool(&connection).await {
        tracing::warn!("⚠️  Failed to warm up connection pool: {}", e);
    } else {
        info!("🔥 Database connection pool warmed up successfully");
    }

    Ok(connection)
}

/// 预热数据库连接池
async fn warm_up_connection_pool(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::Statement;

    // 执行一个简单的查询来预热连接池
    let stmt = Statement::from_string(sea_orm::DatabaseBackend::MySql, "SELECT 1".to_owned());

    // 执行多次查询以确保连接池中的最小连接数都被创建
    for i in 1..=3 {
        match db.execute(stmt.clone()).await {
            Ok(_) => {
                tracing::debug!("🔥 Connection pool warm-up query {} completed", i);
            }
            Err(e) => {
                tracing::warn!("⚠️  Connection pool warm-up query {} failed: {}", i, e);
                return Err(e);
            }
        }
    }

    Ok(())
}
