use anyhow::Result;
use chrono::{ Duration, Utc };
use jsonwebtoken::{ decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation };
use serde::{ Deserialize, Serialize };

use crate::config::Config;
use crate::services::redis::RedisService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 用户名
    pub sub: String,
    /// 用户ID
    pub id: i32,
    /// 过期时间
    pub exp: usize,
}

impl Claims {
    pub fn new(user_id: i32, username: String, exp: Option<usize>) -> Self {
        if let Some(exp) = exp {
            Self {
                sub: username,
                id: user_id,
                exp,
            }
        } else {
            Self {
                sub: username,
                id: user_id,
                exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtData {
    pub user_id: i32,
    pub username: String,
}

pub struct AuthService;

impl AuthService {
    /// 创建访问令牌
    pub fn create_access_token(data: &JwtData, config: &Config) -> Result<String> {
        let now = Utc::now();
        let exp = (now + Duration::days(30)).timestamp() as usize;

        let claims = Claims {
            sub: data.username.clone(),
            exp,
            id: data.user_id,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt.secret.as_ref())
        )?;

        Ok(token)
    }

    /// 验证令牌
    pub async fn verify_token(token: &str, config: &Config) -> Result<Claims, String> {
        // 检查令牌是否在黑名单中
        if Self::is_token_blacklisted(token).await.unwrap_or(false) {
            return Err("Token is blacklisted".to_string());
        }

        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        match
            decode::<Claims>(
                token,
                &DecodingKey::from_secret(config.jwt.secret.as_ref()),
                &validation
            )
        {
            Ok(token_data) => Ok(token_data.claims),
            Err(err) =>
                match err.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        Err("Token has expired".to_string())
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidToken =>
                        Err("Invalid token".to_string()),
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                        Err("Invalid token signature".to_string())
                    }
                    _ => Err("Token validation failed".to_string()),
                }
        }
    }

    /// 将令牌加入黑名单
    pub async fn blacklist_token(token: &str) -> Result<()> {
        let redis = RedisService::instance().ok_or_else(||
            anyhow::anyhow!("Redis service not initialized")
        )?;

        let key = format!("token:invalid:{}", token);
        redis.set_ex(&key, "1", 86400).await?;

        tracing::info!("Token added to blacklist with TTL: {} seconds", 86400);
        Ok(())
    }

    /// 检查令牌是否在黑名单中
    pub async fn is_token_blacklisted(token: &str) -> Result<bool> {
        let redis = RedisService::instance().ok_or_else(||
            anyhow::anyhow!("Redis service not initialized")
        )?;

        let key = format!("token:invalid:{}", token);
        redis.exists(&key).await
    }
}
