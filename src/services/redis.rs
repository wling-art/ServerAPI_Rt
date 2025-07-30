use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::{Client, RedisResult};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::error;

use crate::config::RedisConfig;

/// Redis 服务，管理连接池和基本操作
pub struct RedisService {
    manager: ConnectionManager,
}

// 全局 Redis 实例
static REDIS_INSTANCE: OnceCell<Arc<RedisService>> = OnceCell::const_new();

impl RedisService {
    /// 初始化 Redis 连接
    pub async fn init(config: RedisConfig) -> Result<()> {
        let redis_url = if config.password.as_ref().is_some_and(|p| !p.is_empty()) {
            format!(
                "redis://:{}@{}:{}",
                config.password.as_ref().unwrap(),
                config.host,
                config.port
            )
        } else {
            format!("redis://{}:{}", config.host, config.port)
        };

        tracing::info!("连接到 Redis: {}:{}", config.host, config.port);

        let client = Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;

        let service = Arc::new(RedisService { manager });

        // 测试连接
        service.ping().await?;
        tracing::info!("✅ Redis 连接成功");

        REDIS_INSTANCE
            .set(service)
            .map_err(|_| anyhow::anyhow!("初始化 Redis 实例失败"))?;

        Ok(())
    }

    /// 获取全局 Redis 实例
    pub fn instance() -> Option<Arc<RedisService>> {
        REDIS_INSTANCE.get().cloned()
    }

    /// 测试连接
    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.manager.clone();
        let result: RedisResult<String> = redis::cmd("PING").query_async(&mut conn).await;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Redis ping 失败: {}", e)),
        }
    }

    /// 设置键值对
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let mut conn = self.manager.clone();
        let result: RedisResult<()> = redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut conn)
            .await;

        result.map_err(|e| anyhow::anyhow!("Redis SET 失败: {}", e))
    }

    /// 设置键值对，带过期时间（秒）
    pub async fn set_ex(&self, key: &str, value: &str, expire_seconds: u64) -> Result<()> {
        let mut conn = self.manager.clone();
        let result: RedisResult<()> = redis::cmd("SETEX")
            .arg(key)
            .arg(expire_seconds)
            .arg(value)
            .query_async(&mut conn)
            .await;

        result.map_err(|e| anyhow::anyhow!("Redis SETEX 失败: {}", e))
    }

    /// 获取键的值
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.manager.clone();
        let result: RedisResult<Option<String>> =
            redis::cmd("GET").arg(key).query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis GET 失败: {}", e))
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.manager.clone();
        let result: RedisResult<bool> = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis EXISTS 失败: {}", e))
    }

    /// 批量检查多个键是否存在
    pub async fn batch_exists(&self, keys: &[String]) -> Result<Vec<bool>> {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let mut results = Vec::with_capacity(keys.len());
        let mut conn = self.manager.clone();

        for key in keys {
            let result: RedisResult<bool> =
                redis::cmd("EXISTS").arg(key).query_async(&mut conn).await;

            match result {
                Ok(exists) => results.push(exists),
                Err(e) => {
                    error!("检查键 {} 是否存在时失败: {}", key, e);
                    results.push(false);
                }
            }
        }

        Ok(results)
    }

    /// 删除键
    pub async fn del(&self, key: &str) -> Result<()> {
        let mut conn = self.manager.clone();
        let result: RedisResult<()> = redis::cmd("DEL").arg(key).query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis DEL 失败: {}", e))
    }

    /// 批量删除键
    pub async fn batch_del(&self, keys: &[String]) -> Result<u64> {
        if keys.is_empty() {
            return Ok(0);
        }

        let mut conn = self.manager.clone();
        let mut cmd = redis::cmd("DEL");

        for key in keys {
            cmd.arg(key);
        }

        let result: RedisResult<u64> = cmd.query_async(&mut conn).await;
        result.map_err(|e| anyhow::anyhow!("Redis 批量 DEL 失败: {}", e))
    }

    /// 获取键的剩余过期时间（秒）
    pub async fn ttl(&self, key: &str) -> Result<i64> {
        let mut conn = self.manager.clone();
        let result: RedisResult<i64> = redis::cmd("TTL").arg(key).query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis TTL 失败: {}", e))
    }

    /// 设置键的过期时间
    pub async fn expire(&self, key: &str, seconds: u64) -> Result<bool> {
        let mut conn = self.manager.clone();
        let result: RedisResult<bool> = redis::cmd("EXPIRE")
            .arg(key)
            .arg(seconds)
            .query_async(&mut conn)
            .await;

        result.map_err(|e| anyhow::anyhow!("Redis EXPIRE 失败: {}", e))
    }

    /// 批量删除匹配模式的键
    pub async fn del_pattern(&self, pattern: &str) -> Result<u64> {
        let keys = self.scan_keys(pattern).await?;

        if keys.is_empty() {
            return Ok(0);
        }

        self.batch_del(&keys).await
    }

    /// 使用 SCAN 扫描匹配模式的键
    pub async fn scan_keys(&self, pattern: &str) -> Result<Vec<String>> {
        let mut conn = self.manager.clone();
        let mut cursor = 0u64;
        let mut all_keys = Vec::new();

        loop {
            let result: RedisResult<(u64, Vec<String>)> = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100) // 每次扫描 100 个键
                .query_async(&mut conn)
                .await;

            match result {
                Ok((next_cursor, keys)) => {
                    all_keys.extend(keys);
                    cursor = next_cursor;
                    if cursor == 0 {
                        break; // 扫描完成
                    }
                }
                Err(e) => return Err(anyhow::anyhow!("Redis SCAN 失败: {}", e)),
            }
        }

        Ok(all_keys)
    }

    /// 原子性地设置键值，仅当键不存在时
    pub async fn set_nx(&self, key: &str, value: &str) -> Result<bool> {
        let mut conn = self.manager.clone();
        let result: RedisResult<bool> = redis::cmd("SETNX")
            .arg(key)
            .arg(value)
            .query_async(&mut conn)
            .await;

        result.map_err(|e| anyhow::anyhow!("Redis SETNX 失败: {}", e))
    }

    /// 原子性地设置键值和过期时间，仅当键不存在时
    pub async fn set_nx_ex(&self, key: &str, value: &str, expire_seconds: u64) -> Result<bool> {
        let mut conn = self.manager.clone();
        let result: RedisResult<Option<String>> = redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(expire_seconds)
            .arg("NX")
            .query_async(&mut conn)
            .await;

        match result {
            Ok(Some(_)) => Ok(true), // 设置成功
            Ok(None) => Ok(false),   // 键已存在，设置失败
            Err(e) => Err(anyhow::anyhow!("Redis SET NX EX 失败: {}", e)),
        }
    }

    /// 获取 Redis 信息
    pub async fn info(&self) -> Result<String> {
        let mut conn = self.manager.clone();
        let result: RedisResult<String> = redis::cmd("INFO").query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis INFO 失败: {}", e))
    }

    /// 获取数据库大小
    pub async fn dbsize(&self) -> Result<u64> {
        let mut conn = self.manager.clone();
        let result: RedisResult<u64> = redis::cmd("DBSIZE").query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis DBSIZE 失败: {}", e))
    }
}

// 实现健康检查
impl RedisService {
    /// 健康检查
    pub async fn health_check(&self) -> Result<RedisHealthStatus> {
        let start = std::time::Instant::now();

        match self.ping().await {
            Ok(_) => {
                let latency = start.elapsed();
                let dbsize = self.dbsize().await.unwrap_or(0);

                Ok(RedisHealthStatus {
                    connected: true,
                    latency_ms: latency.as_millis() as u64,
                    dbsize,
                    error: None,
                })
            }
            Err(e) => Ok(RedisHealthStatus {
                connected: false,
                latency_ms: 0,
                dbsize: 0,
                error: Some(e.to_string()),
            }),
        }
    }
}

/// Redis 健康状态
#[derive(Debug, Clone)]
pub struct RedisHealthStatus {
    pub connected: bool,
    pub latency_ms: u64,
    pub dbsize: u64,
    pub error: Option<String>,
}
