use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::{Client, RedisResult};
use std::sync::Arc;
use tokio::sync::OnceCell;

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
        let redis_url = format!("redis://{}:{}", config.host, config.port);
        tracing::info!("🔗 连接到 Redis: {}:{}", config.host, config.port);

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

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.manager.clone();
        let result: RedisResult<bool> = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis EXISTS 失败: {}", e))
    }

    /// 删除键
    pub async fn del(&self, key: &str) -> Result<()> {
        let mut conn = self.manager.clone();
        let result: RedisResult<()> = redis::cmd("DEL").arg(key).query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis DEL 失败: {}", e))
    }

    /// 获取键的剩余过期时间（秒）
    pub async fn ttl(&self, key: &str) -> Result<i64> {
        let mut conn = self.manager.clone();
        let result: RedisResult<i64> = redis::cmd("TTL").arg(key).query_async(&mut conn).await;

        result.map_err(|e| anyhow::anyhow!("Redis TTL 失败: {}", e))
    }

    /// 批量删除匹配模式的键
    pub async fn del_pattern(&self, pattern: &str) -> Result<u64> {
        let mut conn = self.manager.clone();

        // 首先使用 SCAN 找到匹配的键
        let keys: RedisResult<Vec<String>> =
            redis::cmd("KEYS").arg(pattern).query_async(&mut conn).await;

        match keys {
            Ok(key_list) => {
                if key_list.is_empty() {
                    return Ok(0);
                }

                let mut cmd = redis::cmd("DEL");
                for key in &key_list {
                    cmd.arg(key);
                }

                let deleted: RedisResult<u64> = cmd.query_async(&mut conn).await;
                deleted.map_err(|e| anyhow::anyhow!("Redis DEL 匹配失败: {}", e))
            }
            Err(e) => Err(anyhow::anyhow!("Redis KEYS 失败: {}", e)),
        }
    }
}
