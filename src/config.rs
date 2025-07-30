use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub redis: RedisConfig,
    pub s3: S3Config,
    pub email: EmailConfig,
    pub meilisearch: MeilisearchConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub min_connections: u32,
    pub max_connections: u32,
    pub connect_timeout: u64,
    pub acquire_timeout: u64,
    pub idle_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct S3Config {
    pub endpoint_url: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MeilisearchConfig {
    pub url: String,
    pub api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database = DatabaseConfig {
            url: std::env::var("DATABASE_URL")?,
            min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            connect_timeout: std::env::var("DB_CONNECT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            acquire_timeout: std::env::var("DB_ACQUIRE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            idle_timeout: std::env::var("DB_IDLE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600),
        };

        let server = ServerConfig {
            host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
        };

        let jwt = JwtConfig {
            secret: std::env::var("JWT_SECRET")?,
            expiration: std::env::var("JWT_EXPIRATION")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30 * 24 * 60 * 60),
        };

        let redis = RedisConfig {
            host: std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("REDIS_PORT")
                .unwrap_or_else(|_| "6379".to_string())
                .parse()?,
            password: std::env::var("REDIS_PASSWORD").ok(),
        };

        let s3 = S3Config {
            endpoint_url: std::env::var("S3_ENDPOINT_URL")?,
            access_key: std::env::var("S3_ACCESS_KEY")?,
            secret_key: std::env::var("S3_SECRET_KEY")?,
            bucket: std::env::var("S3_BUCKET")?,
        };

        let email = EmailConfig {
            smtp_server: std::env::var("SMTP_SERVER")?,
            smtp_port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "465".to_string())
                .parse()?,
            smtp_username: std::env::var("SMTP_USERNAME")?,
            smtp_password: std::env::var("SMTP_PASSWORD")?,
        };

        let meilisearch = MeilisearchConfig {
            url: std::env::var("MEILISEARCH_URL")?,
            api_key: std::env::var("MEILISEARCH_API_KEY")?,
        };

        Ok(Config {
            database,
            server,
            jwt,
            redis,
            s3,
            email,
            meilisearch,
        })
    }
}
