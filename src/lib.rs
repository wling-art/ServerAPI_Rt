pub mod config;
pub mod entities;
pub mod errors;
pub mod handlers;
pub mod logging;
pub mod middleware;
pub mod schemas;
pub mod services;
use anyhow::Result;
use std::sync::Arc;

use crate::config::Config;
use crate::handlers::search;
use crate::handlers::{auth, servers};
use crate::middleware::{auth::optional_auth_middleware, simple_http_logging_middleware};
use crate::services::auth::SecurityAddon;
use crate::services::database::{establish_connection, DatabaseConnection};
use axum::routing::post;
use axum::{
    middleware as axum_middleware,
    routing::{delete, get},
    Router,
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        servers::list_servers,
        servers::get_server_detail,
        servers::update_server,
        servers::get_server_managers,
        servers::get_server_gallery,
        servers::upload_gallery_image,
        servers::delete_gallery_image,
        servers::get_total_players,
        auth::login,
        auth::logout,
        auth::register,
        auth::register_email_code,
        search::search_server
    ),
    components(
        schemas(
            schemas::servers::ServerListResponse,
            schemas::servers::ApiServerType,
            schemas::servers::ServerDetail,
            schemas::servers::ServerStats,
            schemas::servers::ApiAuthMode,
            schemas::servers::Motd,
            schemas::servers::UpdateServerRequest,
            schemas::servers::ServerManagersResponse,
            schemas::servers::ManagerInfo,
            schemas::servers::ServerGallery,
            schemas::servers::GalleryImage,
            schemas::servers::GalleryImageRequest,
            schemas::servers::SuccessResponse,
            schemas::servers::ServerTotalPlayers,
            schemas::auth::AuthToken,
            schemas::auth::UserRegisterData,
            schemas::search::SearchParams,
            schemas::search::ServerResult,
            schemas::search::SearchResponse,
            entities::server::AuthModeEnum,
            entities::server::ServerTypeEnum,
            errors::ApiErrorResponse,
            errors::ApiError
        )
    ),
    modifiers(&SecurityAddon),
    tags((name = "servers", description = "Server management endpoints"))
)]
pub struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DatabaseConnection,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let config = Arc::new(Config::from_env()?);
        let db = match establish_connection(&config.database).await {
            Ok(db) => {
                tracing::info!("数据库初始化成功");
                db
            }
            Err(e) => {
                tracing::error!("数据库初始化失败: {}", e);
                return Err(e.into());
            }
        };
        Ok(Self { config, db })
    }
}

pub fn create_app(app_state: AppState) -> Router {
    let server_router = Router::new()
        // Server routes with optional authentication
        .route("/", get(servers::list_servers))
        .route("/players", get(servers::get_total_players))
        .route(
            "/{server_id}",
            get(servers::get_server_detail).put(servers::update_server),
        )
        .route("/{server_id}/managers", get(servers::get_server_managers))
        .route(
            "/{server_id}/gallery",
            get(servers::get_server_gallery).post(servers::upload_gallery_image),
        )
        .route(
            "/{server_id}/gallery/{image_id}",
            delete(servers::delete_gallery_image),
        );
    let auth_router = Router::new()
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/register/email-code", post(auth::register_email_code))
        .route("/register", post(auth::register));
    let search_router = Router::new().route("/", get(search::search_server));

    Router::new()
        .nest("/v2/servers", server_router)
        .nest("/v2/auth", auth_router)
        .nest("/v2/search", search_router)
        // Health check
        .route("/health", get(|| async { "OK" }))
        // Swagger UI
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        // CORS configuration
        .layer(CorsLayer::permissive())
        // Add HTTP logging middleware
        .layer(axum_middleware::from_fn(simple_http_logging_middleware))
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            optional_auth_middleware,
        ))
        .with_state(app_state)
}
