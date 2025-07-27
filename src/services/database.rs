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

    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_secs(config.connect_timeout))
        .acquire_timeout(Duration::from_secs(config.acquire_timeout))
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .max_lifetime(Duration::from_secs(28800))
        .sqlx_logging(false);

    info!(
        "🔗 配置数据库连接池: 最小连接数={}, 最大连接数={}",
        config.min_connections, config.max_connections
    );

    let db = Database::connect(opt).await?;
    let connection = Arc::new(db);

    if let Err(e) = warm_up_connection_pool(&connection).await {
        tracing::warn!("⚠️  连接池预热失败: {}", e);
    } else {
        info!("🔥 数据库连接池预热成功");
    }

    Ok(connection)
}

async fn warm_up_connection_pool(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::Statement;

    let stmt = Statement::from_string(sea_orm::DatabaseBackend::MySql, "SELECT 1".to_owned());

    for i in 1..=3 {
        match db.execute(stmt.clone()).await {
            Ok(_) => {
                tracing::debug!("🔥 连接池预热查询 {} 完成", i);
            }
            Err(e) => {
                tracing::warn!("⚠️  连接池预热查询 {} 失败: {}", i, e);
                return Err(e);
            }
        }
    }

    Ok(())
}
