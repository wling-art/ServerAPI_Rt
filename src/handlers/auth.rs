use axum::{extract::State, http::HeaderMap, Json};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{
    config::Config,
    entities::user,
    errors::{ApiError, ApiErrorResponse, ApiResult},
    schemas::auth::AuthToken,
    services::{
        auth::{AuthService, JwtData},
        database::DatabaseConnection,
    },
};
use bcrypt::verify;

#[derive(Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct UserLoginData {
    /// 用户名或邮箱
    username_or_email: String,
    /// 密码
    password: String,
}

fn get_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                let ip = first_ip.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    // 其次从 X-Real-IP 获取
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if !ip_str.is_empty() {
                return Some(ip_str.to_string());
            }
        }
    }

    // 从 X-Forwarded-Host 获取
    if let Some(forwarded_host) = headers.get("x-forwarded-host") {
        if let Ok(host_str) = forwarded_host.to_str() {
            if !host_str.is_empty() {
                return Some(host_str.to_string());
            }
        }
    }

    None
}

/// 用户登陆，返回 JWT token
#[utoipa::path(
    post,
    path = "/v2/auth/login",
    summary = "用户登录",
    description = "使用用户名或邮箱和密码进行登录，成功后返回 JWT 访问令牌",
    tag = "auth",
    responses(
        (status = 200, description = "登录成功", body = AuthToken),
        (status = 400, description = "用户名或密码不能为空", body = ApiErrorResponse),
        (status = 401, description = "用户不存在", body = ApiErrorResponse),
        (status = 500, description = "服务器错误", body = ApiErrorResponse)
    ),
    params(
        ("username_or_email" = String, description = "用户名或邮箱"),
        ("password" = String, description = "密码")
    )
)]
pub async fn login(
    headers: HeaderMap,
    State(db): State<DatabaseConnection>,
    Json(user_data): Json<UserLoginData>,
) -> ApiResult<Json<AuthToken>> {
    // 开始校验
    if user_data.username_or_email.is_empty() || user_data.password.is_empty() {
        return Err(ApiError::BadRequest("用户名或密码不能为空".to_string()));
    };

    let user = if user_data.username_or_email.contains('@') {
        user::Entity::find()
            .filter(user::Column::Email.eq(user_data.username_or_email.clone()))
            .one(&*db)
            .await?
            .ok_or(ApiError::Unauthorized("用户不存在".to_string()))?
    } else {
        user::Entity::find()
            .filter(user::Column::Username.eq(user_data.username_or_email.clone()))
            .one(&*db)
            .await?
            .ok_or(ApiError::Unauthorized("用户不存在".to_string()))?
    };
    let config = Config::from_env()?;

    match verify(&user_data.password, &user.hashed_password) {
        Ok(true) => {
            AuthService::update_last_login(&db, user.id, get_ip(&headers)).await?;
            let jwt_data = JwtData {
                user_id: user.id,
                username: user.username,
            };
            let token = AuthService::create_access_token(&jwt_data, &config)?;
            Ok(Json(AuthToken {
                access_token: token,
                expires_in: config.jwt.expiration,
            }))
        }
        Ok(false) => Err(ApiError::Unauthorized("密码错误".to_string())),
        Err(_) => Err(ApiError::InternalServerError("密码校验失败".to_string())),
    }
}
