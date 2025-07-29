use axum::{extract::State, http::HeaderMap, Extension, Json};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio::task;
use validator::Validate;

use crate::{
    entities::users,
    errors::{ApiError, ApiErrorResponse, ApiResult},
    middleware::UserClaims,
    schemas::{
        auth::{AuthToken, UserLoginData, UserRegisterByEmailData},
        servers::SuccessResponse,
    },
    services::auth::{AuthService, JwtData},
    AppState,
};
use anyhow::Context;
use bcrypt::verify;

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
    )
)]
pub async fn login(
    headers: HeaderMap,
    State(app_state): State<AppState>,
    Json(user_data): Json<UserLoginData>,
) -> ApiResult<Json<AuthToken>> {
    if user_data.username_or_email.is_empty() || user_data.password.is_empty() {
        return Err(ApiError::BadRequest("用户名或密码不能为空".to_string()));
    }

    let config = &app_state.config;
    let db = &app_state.db;

    let (user_result, client_ip) = tokio::join!(
        async {
            if user_data.username_or_email.contains('@') {
                users::Entity::find()
                    .filter(users::Column::Email.eq(&user_data.username_or_email))
                    .one(db.as_ref())
                    .await
            } else {
                users::Entity::find()
                    .filter(users::Column::Username.eq(&user_data.username_or_email))
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

    // TODO: 写一个 bcrypt 转 10 cost，不然 12 cost 太慢了

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
                    eprintln!("更新最后登录时间失败: {e:?}");
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

#[utoipa::path(
    post,
    path = "/v2/auth/logout",
    summary = "用户登出",
    description = "登出当前用户，清除 JWT 访问令牌",
    tag = "auth",
    responses(
        (status = 200, description = "登出成功", body = SuccessResponse),
        (status = 401, description = "未登录或令牌无效", body = ApiErrorResponse),
        (status = 500, description = "服务器错误", body = ApiErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn logout(
    State(app_state): State<AppState>,
    user_claims: Option<Extension<UserClaims>>,
) -> ApiResult<Json<SuccessResponse>> {
    if let Some(claims) = user_claims {
        AuthService::blacklist_token(&claims.raw_token, &app_state.config).await?;

        Ok(Json(SuccessResponse {
            message: "登出成功".to_string(),
        }))
    } else {
        Err(ApiError::Unauthorized("未登录或令牌无效".to_string()))
    }
}

/// 注册
#[utoipa::path(
    post,
    path = "/v2/auth/register/email-code",
    summary = "使用邮箱注册用户",
    description = "使用邮箱注册用户，发送验证码到用户邮箱",
    tag = "auth",
    responses(
        (status = 200, description = "注册成功", body = SuccessResponse),
        (status = 400, description = "请求数据不合法", body = ApiErrorResponse),
        (status = 400, description = "用户已存在", body = ApiErrorResponse),
        (status = 500, description = "服务器错误", body = ApiErrorResponse)
    )
)]
pub async fn register_email_code(
    State(app_state): State<AppState>,
    Json(user_data): Json<UserRegisterByEmailData>,
) -> ApiResult<Json<SuccessResponse>> {
    if user_data.email.is_empty() {
        return Err(ApiError::BadRequest("邮箱不能为空".to_string()));
    }
    if user_data.validate().is_err() {
        return Err(ApiError::BadRequest("请求数据不合法".to_string()));
    }

    let user_exists = users::Entity::find()
        .filter(users::Column::Email.eq(&user_data.email))
        .one(app_state.db.as_ref())
        .await
        .map(|user| user.is_some())
        .context("检查用户是否存在失败")?;

    if user_exists {
        return Err(ApiError::BadRequest("用户已存在".to_string()));
    }

    AuthService::send_email_code(&user_data.email, &app_state.config)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("发送验证码失败: {e}")))?;

    Ok(Json(SuccessResponse {
        message: format!("验证码已发送到 {}", user_data.email),
    }))
}
