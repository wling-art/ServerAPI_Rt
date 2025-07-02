use sea_orm::{Database, DatabaseConnection as SeaOrmDatabaseConnection, DbErr};
use std::sync::Arc;

pub type DatabaseConnection = Arc<SeaOrmDatabaseConnection>;

pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(database_url).await?;
    Ok(Arc::new(db))
}
