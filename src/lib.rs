pub mod config;
pub mod entities;
pub mod errors;
pub mod handlers;
pub mod logging;
pub mod middleware;
pub mod schemas;
pub mod services;

use crate::config::AppState;
use crate::handlers::{auth, servers};
use crate::middleware::{auth::optional_auth_middleware, simple_http_logging_middleware};
use crate::services::auth::SecurityAddon;
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
        auth::login
    ),
    components(
        schemas(
            schemas::servers::ServerListResponse,
            schemas::servers::ApiServerType,
            schemas::servers::ServerDetail,
            schemas::servers::ServerStatus,
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
            entities::server::AuthModeEnum,
            entities::server::ServerTypeEnum,
            crate::errors::ApiErrorResponse,
            crate::errors::ApiError
        )
    ),
    modifiers(&SecurityAddon),
    tags((name = "servers", description = "Server management endpoints"))
)]
pub struct ApiDoc;

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
    let auth_router = Router::new().route("/login", post(auth::login));

    Router::new()
        .nest("/v2/servers", server_router)
        .nest("/v2/auth", auth_router)
        // Health check
        .route("/health", get(|| async { "OK" }))
        // Swagger UI
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        // CORS configuration
        .layer(CorsLayer::permissive())
        // Add HTTP logging middleware
        .layer(axum_middleware::from_fn(simple_http_logging_middleware))
        .layer(axum_middleware::from_fn(optional_auth_middleware))
        .with_state(app_state)
}
