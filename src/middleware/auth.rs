use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    errors::ApiError,
    services::auth::{AuthService, Claims},
    AppState,
};

#[derive(Debug, Clone)]
pub struct UserClaims {
    pub claims: Claims,
    pub raw_token: String,
}

fn extract_bearer_token(req: &Request) -> Option<String> {
    req.headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "))
        .map(|token| token.to_string())
}

pub async fn optional_auth_middleware(
    State(app_state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(token) = extract_bearer_token(&req) {
        match AuthService::verify_token(&token, &app_state.config).await {
            Ok(claims) => {
                req.extensions_mut().insert(UserClaims {
                    claims,
                    raw_token: token,
                });
            }
            Err(_) => {
                return ApiError::Unauthorized("无效的 Token".to_string()).into_response();
            }
        }
    }

    next.run(req).await
}
