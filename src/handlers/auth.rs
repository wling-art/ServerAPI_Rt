use axum::{extract::State, http::HeaderMap, Json};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use tokio::task;

use crate::{
    config::AppState,
    entities::user,
    errors::{ApiError, ApiErrorResponse, ApiResult},
    schemas::auth::AuthToken,
    services::auth::{AuthService, JwtData},
};
use bcrypt::verify;

#[derive(Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct UserLoginData {
    username_or_email: String,
    password: String,
}

fn get_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            headers
                .get("x-forwarded-host")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
        })
}

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
    State(app_state): State<AppState>,
    Json(user_data): Json<UserLoginData>,
) -> ApiResult<Json<AuthToken>> {
    use std::time::Instant;

    if user_data.username_or_email.is_empty() || user_data.password.is_empty() {
        return Err(ApiError::BadRequest("用户名或密码不能为空".to_string()));
    }

    let config = &app_state.config;
    let db = &app_state.db;

    let (user_result, client_ip) = tokio::join!(
        async {
            if user_data.username_or_email.contains('@') {
                user::Entity::find()
                    .filter(user::Column::Email.eq(&user_data.username_or_email))
                    .one(db.as_ref())
                    .await
            } else {
                user::Entity::find()
                    .filter(user::Column::Username.eq(&user_data.username_or_email))
                    .one(db.as_ref())
                    .await
            }
        },
        async { get_ip(&headers) }
    );

    let user = user_result?.ok_or(ApiError::Unauthorized("用户不存在".to_string()))?;

    let password = user_data.password;
    let hashed_password = user.hashed_password.clone();
    let user_id = user.id;
    let username = user.username.clone();

    let verify_result = task::spawn_blocking(move || verify(&password, &hashed_password)) // 煞笔 bcrypt 真他妈慢
        .await
        .map_err(|_| ApiError::InternalServerError("密码校验任务失败".to_string()))?;

    match verify_result {
        Ok(true) => {
            let jwt_data = JwtData {
                user_id,
                username: username.clone(),
            };
            let token = AuthService::create_access_token(&jwt_data, config)?;

            let db_clone = db.clone();
            tokio::spawn(async move {
                if let Err(e) = AuthService::update_last_login(&db_clone, user_id, client_ip).await
                {
                    eprintln!("更新最后登录时间失败: {:?}", e);
                }
            });

            Ok(Json(AuthToken {
                access_token: token,
                expires_in: config.jwt.expiration,
            }))
        }
        Ok(false) => Err(ApiError::Unauthorized("密码错误".to_string())),
        Err(_) => Err(ApiError::InternalServerError("密码校验失败".to_string())),
    }
}
