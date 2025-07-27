use crate::{
    errors::{ApiError, ApiErrorResponse, ApiResult},
    schemas::servers::{
        GalleryImageRequest, GalleryImageSchema, ServerDetail, ServerGallery, ServerListResponse,
        ServerManagersResponse, ServerTotalPlayers, SuccessResponse, UpdateServerRequest,
    },
    services::{auth::Claims, server::ServerService},
    AppState,
};
use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use axum_typed_multipart::TypedMultipart;
use serde::Deserialize;

fn default_is_member() -> bool {
    true
}
fn default_page_size() -> u64 {
    5
}
fn default_page() -> u64 {
    1
}

#[derive(Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ListQuery {
    /// 页码
    #[schema(example = 1, default = 1)]
    #[serde(default = "default_page")]
    pub page: u64,
    /// 每页数量
    #[schema(example = 5, default = 5)]
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    /// 是否为成员服务器
    #[schema(example = true, default = true)]
    #[serde(default = "default_is_member")]
    pub is_member: bool,
    /// 服务器类型-筛选
    #[schema(example = json!(["JAVA", "BEDROCK"]))]
    #[serde(default)]
    pub r#type: Option<Vec<String>>,
    /// 认证方式-筛选
    #[schema(example = json!(["OFFLINE!", "OFFICIAL", "YGGDRASIL"]))]
    #[serde(default)]
    pub auth_mode: Option<Vec<String>>,
    /// 标签
    #[schema(example = json!(["生存", "PVP"]))]
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// 随机种子，固定分页用
    #[schema(example = 114514, default = 114514)]
    #[serde(default)]
    pub seed: Option<i64>,
}

#[derive(Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ServerDetailQuery {
    /// 是否返回完整信息(需要登录)
    #[schema(example = false, default = false)]
    #[serde(default)]
    pub full_info: Option<bool>,
}

/// 获取服务器列表
#[utoipa::path(
    get,
    path = "/v2/servers",
    responses(
        (
            status = 200,
            description = "成功获取服务器列表",
            body = ServerListResponse,
        ),
        (
            status = 400,
            description = "请求参数错误",
            body = ApiErrorResponse,
            example = json!({
             "error": "page 与 page_size 不能小于 1",
             "status": 400
         }),
        )
    ),
    tag = "servers",
    params(ListQuery),
    security(
        (),
        ("bearer_auth" = [])
    )
)]
pub async fn list_servers(
    State(app_state): State<AppState>,
    Query(query): Query<ListQuery>,
    user_claims: Option<Extension<Claims>>,
) -> ApiResult<Json<ServerListResponse>> {
    if query.page < 1 || query.page_size < 1 {
        return Err(ApiError::BadRequest(
            "page 与 page_size 不能小于 1".to_string(),
        ));
    }
    let db = &app_state.db;
    let user_id = user_claims.map(|Extension(claims)| claims.id);

    let result = ServerService::get_servers_with_filters(db, user_id, &query).await?;

    let total = result.total;
    let total_pages = ((total as f64) / (query.page_size as f64)).ceil() as i64;

    Ok(Json(ServerListResponse {
        data: result.data,
        total,
        total_pages,
    }))
}

/// 获取特定服务器的详细信息
#[utoipa::path(
    get,
    path = "/v2/servers/{server_id}",
    responses(
        (status = 200,
         description = "成功获取服务器详细信息",
         body = ServerDetail,
        ),
        (status = 404,
         description = "服务器不存在",
         body = ApiErrorResponse,
         example = json!(serde_json::to_value(ApiErrorResponse {
             error: "服务器不存在".to_string(),
             status: 404,
         }).unwrap())
        ),
        (status = 401,
         description = "未登录或无权限访问",
         body = ApiErrorResponse,
         example = json!(serde_json::to_value(ApiErrorResponse {
             error: "未登录，禁止访问".to_string(),
             status: 401,
         }).unwrap())
        )
    ),
    tag = "servers",
    params(("server_id" = i32, Path, description = "服务器 ID"),
           ServerDetailQuery),
    security(
        (),
        ("bearer_auth" = [])
    )
)]
pub async fn get_server_detail(
    State(app_state): State<AppState>,
    Path(server_id): Path<i32>,
    Query(query): Query<ServerDetailQuery>,
    user_claims: Option<Extension<Claims>>,
) -> ApiResult<Json<ServerDetail>> {
    let user_id = user_claims.map(|Extension(claims)| claims.id);

    let full_info = query.full_info.unwrap_or(false);
    let db = &app_state.db;

    let result = ServerService::get_server_detail(db, user_id, server_id, full_info).await?;

    Ok(Json(result))
}

/// 更新对应服务器具体信息
#[utoipa::path(
    put,
    path = "/v2/servers/{server_id}",
    request_body(content = UpdateServerRequest, content_type = "multipart/form-data"),
    responses(
        (
            status = 200,
            description = "成功更新服务器信息",
            body = ServerDetail,
        ),
        (
            status = 400,
            description = "无效的请求参数",
            body = ApiErrorResponse,
            examples(
                ("更新字段不能为空" = (value = json!({"error": "更新字段不能为空", "status": 400}))),
                ("tags数量不能超过7个" = (value = json!({"error": "tags 数量不能超过 7 个", "status": 400}))),
                ("tags长度限制为1~4" = (value = json!({"error": "tags 长度限制为 1~4", "status": 400}))),
                ("简介必须大于100字" = (value = json!({"error": "简介必须大于 100 字", "status": 400})))
            ),
        ),
        (
            status = 401,
            description = "未授权",
            body = ApiErrorResponse,
            example = json!({"error": "未授权", "status": 401}),
        ),
        (
            status = 403,
            description = "无权限编辑该服务器",
            body = ApiErrorResponse,
            example = json!({"error": "无权限编辑该服务器", "status": 403}),
        ),
        (
            status = 404,
            description = "未找到该服务器",
            body = ApiErrorResponse,
            example = json!({"error": "未找到该服务器", "status": 404}),
        )
    ),
    tag = "servers",
    params(("server_id" = i32, Path, description = "服务器 ID")),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_server(
    State(app_state): State<AppState>,
    Path(server_id): Path<i32>,
    user_claims: Option<Extension<Claims>>,
    TypedMultipart(update_data): TypedMultipart<UpdateServerRequest>,
) -> ApiResult<Json<ServerDetail>> {
    // 检查用户是否已登录
    let user = user_claims.ok_or_else(|| ApiError::Unauthorized("未授权".to_string()))?;

    // 从环境变量获取 S3 配置
    let s3_config = crate::config::S3Config {
        endpoint_url: std::env::var("S3_ENDPOINT_URL")
            .map_err(|_| ApiError::Internal("S3配置缺失".to_string()))?,
        access_key: std::env::var("S3_ACCESS_KEY")
            .map_err(|_| ApiError::Internal("S3配置缺失".to_string()))?,
        secret_key: std::env::var("S3_SECRET_KEY")
            .map_err(|_| ApiError::Internal("S3配置缺失".to_string()))?,
        bucket: std::env::var("S3_BUCKET")
            .map_err(|_| ApiError::Internal("S3配置缺失".to_string()))?,
    };
    let db = &app_state.db;

    // 调用服务层更新服务器
    let updated_server =
        ServerService::update_server_by_id(db, &s3_config, server_id, update_data, user.id)
            .await?;

    Ok(Json(updated_server))
}

/// 获取服务器管理员列表
#[utoipa::path(
    get,
    path = "/v2/servers/{server_id}/managers",
    responses(
        (
            status = 200,
            description = "成功获取服务器管理员列表",
            body = ServerManagersResponse,
        ),
        (
            status = 404,
            description = "服务器不存在",
            body = ApiErrorResponse,
            example = json!({
                "error": "服务器不存在",
                "status": 404
            }),
        )
    ),
    tag = "servers",
    params(("server_id" = i32, Path, description = "服务器 ID"))
)]
pub async fn get_server_managers(
    State(app_state): State<AppState>,
    Path(server_id): Path<i32>,
) -> ApiResult<Json<ServerManagersResponse>> {
    let db = &app_state.db;
    let result = ServerService::get_server_managers(db, server_id).await?;
    Ok(Json(result))
}

/// 获取服务器相册
#[utoipa::path(
    get,
    path = "/v2/servers/{server_id}/gallery",
    summary = "获取服务器相册",
    description = "获取指定服务器的所有相册图片信息",
    responses(
        (
            status = 200,
            description = "成功获取服务器相册",
            body = ServerGallery,
        ),
        (
            status = 404,
            description = "服务器不存在",
            body = ApiErrorResponse,
            example = json!({
                "error": "服务器不存在",
                "status": 404
            })
        )
    ),
    tag = "servers",
    params(("server_id" = i32, Path, description = "服务器ID"))
)]
pub async fn get_server_gallery(
    State(app_state): State<AppState>,
    Path(server_id): Path<i32>,
) -> ApiResult<Json<ServerGallery>> {
    let db = &app_state.db;
    let result = ServerService::get_server_gallery(db, server_id).await?;
    Ok(Json(result))
}

/// 添加服务器画册图片
#[utoipa::path(
    post,
    path = "/v2/servers/{server_id}/gallery",
    summary = "添加服务器画册图片",
    description = "为指定服务器添加画册图片，需要服务器管理员权限",
    request_body(
        content = GalleryImageRequest,
        content_type = "multipart/form-data"
    ),
    responses(
        (
            status = 201,
            description = "成功添加服务器画册图片",
            body = SuccessResponse,
            example = json!({
                "message": "成功添加服务器画册图片"
            })
        ),
        (
            status = 401,
            description = "无权限操作",
            body = ApiErrorResponse,
            example = json!({
                "error": "未授权",
                "status": 401
            })
        ),
        (
            status = 403,
            description = "权限不足",
            body = ApiErrorResponse,
            example = json!({
                "error": "权限不足，只有服务器管理员可以添加画册图片",
                "status": 403
            })
        ),
        (
            status = 404,
            description = "未找到服务器",
            body = ApiErrorResponse,
            example = json!({
                "error": "服务器不存在",
                "status": 404
            })
        ),
        (
            status = 400,
            description = "请求参数错误",
            body = ApiErrorResponse,
            example = json!({
                "error": "图片文件格式无效",
                "status": 400
            })
        )
    ),
    tag = "servers",
    params(("server_id" = i32, Path, description = "服务器ID")),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn upload_gallery_image(
    State(app_state): State<AppState>,
    Path(server_id): Path<i32>,
    user_claims: Option<Extension<Claims>>,
    TypedMultipart(gallery_data): TypedMultipart<GalleryImageSchema>,
) -> ApiResult<Json<serde_json::Value>> {
    // 检查用户是否登录
    let claims = user_claims
        .ok_or_else(|| ApiError::Unauthorized("未授权".to_string()))?
        .0;
    let db = &app_state.db;

    // 检查用户是否有这个服务器的编辑权
    let has_permission =
        ServerService::has_server_edit_permission(db, claims.id, server_id).await?;
    if !has_permission {
        return Err(ApiError::Forbidden(
            "权限不足，只有服务器管理员可以添加画册图片".to_string(),
        ));
    }

    // 从环境变量获取S3配置
    let config = crate::config::Config::from_env()
        .map_err(|e| ApiError::Internal(format!("配置加载失败: {e}")))?;

    // 添加画册图片
    ServerService::add_gallery_image(db, &config.s3, server_id, &gallery_data).await?;

    Ok(Json(serde_json::json!({
        "message": "成功添加服务器画册图片"
    })))
}

/// 删除服务器画册图片
#[utoipa::path(
    delete,
    path = "/v2/servers/{server_id}/gallery/{image_id}",
    summary = "删除服务器画册图片",
    description = "删除指定服务器的画册图片，需要服务器管理员权限",
    responses(
        (
            status = 200,
            description = "成功删除服务器画册图片",
            body = SuccessResponse,
            example = json!({
                "message": "成功删除服务器画册图片"
            })
        ),
        (
            status = 401,
            description = "无权限操作",
            body = ApiErrorResponse,
            example = json!({
                "error": "未授权",
                "status": 401
            })
        ),
        (
            status = 403,
            description = "权限不足",
            body = ApiErrorResponse,
            example = json!({
                "error": "权限不足，只有服务器管理员可以删除画册图片",
                "status": 403
            })
        ),
        (
            status = 404,
            description = "未找到服务器或图片",
            body = ApiErrorResponse,
            examples(
                ("服务器不存在" = (value = json!({"error": "服务器不存在", "status": 404}))),
                ("图片不存在" = (value = json!({"error": "图片不存在", "status": 404}))),
                ("该服务器没有画册" = (value = json!({"error": "该服务器没有画册", "status": 404})))
            )
        ),
        (
            status = 403,
            description = "图片不属于该服务器",
            body = ApiErrorResponse,
            example = json!({
                "error": "图片不属于该服务器",
                "status": 403
            })
        )
    ),
    tag = "servers",
    params(
        ("server_id" = i32, Path, description = "服务器ID"),
        ("image_id" = i32, Path, description = "图片ID")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_gallery_image(
    State(app_state): State<AppState>,
    Path((server_id, image_id)): Path<(i32, i32)>,
    user_claims: Option<Extension<Claims>>,
) -> ApiResult<Json<serde_json::Value>> {
    // 检查用户是否登录
    let claims = user_claims
        .ok_or_else(|| ApiError::Unauthorized("未授权".to_string()))?
        .0;
    let db = &app_state.db;
    // 检查用户是否有这个服务器的编辑权
    let has_permission =
        ServerService::has_server_edit_permission(db, claims.id, server_id).await?;
    if !has_permission {
        return Err(ApiError::Forbidden(
            "权限不足，只有服务器管理员可以删除画册图片".to_string(),
        ));
    }

    // 从环境变量获取S3配置
    let config = crate::config::Config::from_env()
        .map_err(|e| ApiError::Internal(format!("配置加载失败: {e}")))?;

    // 删除画册图片
    ServerService::delete_gallery_image(db, &config.s3, server_id, image_id).await?;

    Ok(Json(serde_json::json!({
        "message": "成功删除服务器画册图片"
    })))
}

/// 获取所有服务器玩家总数
#[utoipa::path(
    get,
    path = "/v2/servers/players",
    responses(
        (
            status = 200,
            description = "成功获取所有服务器玩家总数",
            body = ServerTotalPlayers,
        ),
        (
            status = 500,
            description = "服务器内部错误",
            body = ApiErrorResponse,
        )
    ),
    tag = "servers"
)]
pub async fn get_total_players(
    State(app_state): State<AppState>,
) -> ApiResult<Json<ServerTotalPlayers>> {
    let db = &app_state.db;
    let result = ServerService::total_players(db).await?;
    Ok(Json(result))
}
