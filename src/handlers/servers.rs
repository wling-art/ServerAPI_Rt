use crate::{
    errors::{ApiError, ApiErrorResponse, ApiResult},
    schemas::servers::{
        ServerDetail, ServerGallery, ServerListResponse, ServerManagersResponse,
        UpdateServerRequest,
    },
    services::{auth::Claims, database::DatabaseConnection, server::ServerService},
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
    params(ListQuery)
)]
pub async fn list_servers(
    State(db): State<DatabaseConnection>,
    Query(query): Query<ListQuery>,
    user_claims: Option<Extension<Claims>>,
) -> ApiResult<Json<ServerListResponse>> {
    if query.page < 1 || query.page_size < 1 {
        return Err(ApiError::BadRequest(
            "page 与 page_size 不能小于 1".to_string(),
        ));
    }

    let user_id = user_claims.map(|Extension(claims)| claims.id);

    let result = ServerService::get_servers_with_filters(&db, user_id, &query).await?;

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
           ServerDetailQuery)
)]
pub async fn get_server_detail(
    State(db): State<DatabaseConnection>,
    Path(server_id): Path<i32>,
    Query(query): Query<ServerDetailQuery>,
    user_claims: Option<Extension<Claims>>,
) -> ApiResult<Json<ServerDetail>> {
    let user_id = user_claims.map(|Extension(claims)| claims.id);

    let full_info = query.full_info.unwrap_or(false);

    let result = ServerService::get_server_detail(&db, user_id, server_id, full_info).await?;

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
    params(("server_id" = i32, Path, description = "服务器 ID"))
)]
pub async fn update_server(
    State(db): State<DatabaseConnection>,
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

    // 调用服务层更新服务器
    let updated_server =
        ServerService::update_server_by_id(&db, &s3_config, server_id, update_data, user.id)
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
    State(db): State<DatabaseConnection>,
    Path(server_id): Path<i32>,
) -> ApiResult<Json<ServerManagersResponse>> {
    let result = ServerService::get_server_managers(&db, server_id).await?;
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
    State(db): State<DatabaseConnection>,
    Path(server_id): Path<i32>,
) -> ApiResult<Json<ServerGallery>> {
    let result = ServerService::get_server_gallery(&db, server_id).await?;
    Ok(Json(result))
}
