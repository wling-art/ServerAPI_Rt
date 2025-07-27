use crate::config::Config;
use crate::entities::user;
use crate::services::redis::RedisService;
use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{ActiveModelTrait, DatabaseConnection};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tracing::error;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify,
};

/// JWT令牌声明结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 用户名（主题）
    pub sub: String,
    /// 用户ID
    pub id: i32,
    /// 过期时间戳
    pub exp: usize,
}

/// JWT数据传输对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtData {
    pub user_id: i32,
    pub username: String,
}

impl Claims {
    /// 创建新的JWT声明
    pub fn new(user_id: i32, username: String, exp: Option<usize>) -> Self {
        let exp = exp.unwrap_or_else(|| (Utc::now() + Duration::hours(24)).timestamp() as usize);

        Self {
            sub: username,
            id: user_id,
            exp,
        }
    }
}

/// OpenAPI安全配置插件
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert(Default::default());
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

/// 认证服务
pub struct AuthService;

impl AuthService {
    /// Redis黑名单键前缀
    const BLACKLIST_PREFIX: &'static str = "token:blacklist";
    /// 默认令牌过期时间（秒）
    const DEFAULT_TTL: u64 = 86400; // 24小时

    /// 创建访问令牌
    ///
    /// # 参数
    /// * `data` - JWT数据
    /// * `config` - 应用配置
    pub fn create_access_token(data: &JwtData, config: &Config) -> Result<String> {
        let exp = (Utc::now() + Duration::days(30)).timestamp() as usize;
        let claims = Claims {
            sub: data.username.clone(),
            id: data.user_id,
            exp,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt.secret.as_ref()),
        )
        .map_err(Into::into)
    }

    /// 验证令牌有效性
    ///
    /// # 参数
    /// * `token` - 待验证的JWT令牌
    /// * `config` - 应用配置
    pub async fn verify_token(token: &str, config: &Config) -> Result<Claims, String> {
        // 解码令牌
        let claims = Self::decode_token(token, config)?;

        // 检查是否过期
        Self::check_token_expiry(&claims)?;

        // 检查黑名单
        Self::check_blacklist(token).await?;

        Ok(claims)
    }

    /// 将令牌加入黑名单
    pub async fn blacklist_token(token: &str, config: &Config) -> Result<()> {
        let redis = Self::get_redis_service()?;
        let ttl = Self::calculate_token_ttl(token, config).unwrap_or(Self::DEFAULT_TTL);
        let key = Self::build_blacklist_key(token);

        redis.set_ex(&key, "1", ttl).await.map_err(|e| {
            error!("令牌黑名单操作失败: {}", e);
            anyhow::anyhow!("令牌黑名单操作失败: {}", e)
        })
    }

    /// 检查令牌是否在黑名单中
    pub async fn is_token_blacklisted(token: &str) -> Result<bool> {
        let redis = Self::get_redis_service()?;
        let key = Self::build_blacklist_key(token);

        redis.exists(&key).await.map_err(|e| {
            error!("检查令牌黑名单失败: {}", e);
            anyhow::anyhow!("Redis查询失败: {}", e)
        })
    }

    /// 批量检查多个令牌的黑名单状态
    pub async fn batch_check_blacklist(tokens: &[String]) -> Result<Vec<bool>> {
        let redis = Self::get_redis_service()?;
        let keys: Vec<String> = tokens
            .iter()
            .map(|token| Self::build_blacklist_key(token))
            .collect();

        redis.batch_exists(&keys).await.map_err(|e| {
            error!("批量检查令牌黑名单失败: {}", e);
            anyhow::anyhow!("批量Redis查询失败: {}", e)
        })
    }

    /// 更新用户最后登录信息
    pub async fn update_last_login(
        db: &DatabaseConnection,
        user_id: i32,
        ip: Option<String>,
    ) -> Result<()> {
        let user = user::ActiveModel {
            id: sea_orm::Set(user_id),
            last_login: sea_orm::Set(Some(Utc::now())),
            last_login_ip: sea_orm::Set(ip),
            ..Default::default()
        };

        user.update(db).await.map(|_| ()).map_err(|e| {
            error!("更新最后登录信息失败: {}", e);
            e.into()
        })
    }

    // ========== 私有辅助方法 ==========

    /// 获取Redis服务实例
    fn get_redis_service() -> Result<std::sync::Arc<RedisService>> {
        RedisService::instance().ok_or_else(|| anyhow::anyhow!("Redis服务未初始化"))
    }

    /// 解码JWT令牌
    fn decode_token(token: &str, config: &Config) -> Result<Claims, String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false; // 手动处理过期验证

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(config.jwt.secret.as_ref()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|err| match err.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidToken => "无效令牌".to_string(),
            jsonwebtoken::errors::ErrorKind::InvalidSignature => "令牌签名无效".to_string(),
            _ => "令牌验证失败".to_string(),
        })
    }

    /// 检查令牌是否过期
    fn check_token_expiry(claims: &Claims) -> Result<(), String> {
        let now = Utc::now().timestamp() as usize;
        if claims.exp < now {
            Err("令牌已过期".to_string())
        } else {
            Ok(())
        }
    }

    /// 检查令牌黑名单状态
    async fn check_blacklist(token: &str) -> Result<(), String> {
        match Self::is_token_blacklisted(token).await {
            Ok(true) => Err("令牌已被吊销".to_string()),
            Ok(false) => Ok(()),
            Err(e) => {
                error!("检查令牌黑名单失败: {}", e);
                Err("服务暂时不可用".to_string())
            }
        }
    }

    /// 构建黑名单Redis键
    fn build_blacklist_key(token: &str) -> String {
        format!("{}:{}", Self::BLACKLIST_PREFIX, Self::hash_token(token))
    }

    /// 计算令牌的剩余TTL
    fn calculate_token_ttl(token: &str, config: &Config) -> Option<u64> {
        let validation = Validation::new(Algorithm::HS256);
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(config.jwt.secret.as_ref()),
            &validation,
        )
        .ok()
        .and_then(|data| {
            let now = Utc::now().timestamp() as usize;
            if data.claims.exp > now {
                Some((data.claims.exp - now) as u64)
            } else {
                None
            }
        })
    }

    /// 对令牌进行哈希处理（避免Redis键过长）
    fn hash_token(token: &str) -> String {
        let mut hasher = DefaultHasher::new();
        token.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
