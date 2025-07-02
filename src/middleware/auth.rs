use axum::{
    extract::{ Request, State },
    http::{ header::AUTHORIZATION },
    middleware::Next,
    response::Response,
};

use crate::{ config::Config, services::{ auth::AuthService, database::DatabaseConnection } };

/// Optional authentication middleware - extracts user info if present but doesn't require it
pub async fn optional_auth_middleware(
    State(_db): State<DatabaseConnection>,
    mut req: Request,
    next: Next
) -> Response {
    // 获取配置
    if let Ok(config) = Config::from_env() {
        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok());

        if let Some(auth_header) = auth_header {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                if let Ok(claims) = AuthService::verify_token(token, &config).await {
                    // Add user info to request extensions if token is valid
                    req.extensions_mut().insert(claims);
                }
            }
        }
    }

    next.run(req).await
}
