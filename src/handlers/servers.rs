use axum::{ extract::{ Path, Query, State, Extension }, Json };
use serde::Deserialize;
use crate::{
    errors::{ ApiError, ApiResult, ApiErrorResponse },
    schemas::servers::{
        ServerDetail,
        ServerListResponse,
        create_example_server_detail,
        create_example_server_list_response,
    },
    services::{ auth::Claims, database::DatabaseConnection, server::ServerService },
};

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

/// 获取服务器列表
#[utoipa::path(
    get,
    path = "/v2/servers",
    responses(
        (
            status = 200,
            description = "成功获取服务器列表",
            body = ServerListResponse,
            example = json!(serde_json::to_value(create_example_server_list_response()).unwrap()),
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
    user_claims: Option<Extension<Claims>>
) -> ApiResult<Json<ServerListResponse>> {
    if query.page < 1 || query.page_size < 1 {
        println!("Invalid page or page_size: page={}, page_size={}", query.page, query.page_size);
        return Err(ApiError::BadRequest("page 与 page_size 不能小于 1".to_string()));
    }

    let user_id = user_claims.map(|Extension(claims)| claims.id);

    let result = ServerService::get_servers_with_filters(&db, user_id, &query).await?;

    let total = result.total;
    let total_pages = ((total as f64) / (query.page_size as f64)).ceil() as i64;

    Ok(
        Json(ServerListResponse {
            data: result.data,
            total,
            total_pages,
        })
    )
}
/// 获取特定服务器的详细信息
#[utoipa::path(
    get,
    path = "/v2/servers/{id}",
    responses(
        (status = 200,
         description = "成功获取服务器详细信息",
         body = ServerDetail,
         example = json!(serde_json::to_value(create_example_server_detail()).unwrap())
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
    params(("id" = u64, Path, description = "服务器 ID"),
            ("full_info" = Option<bool>, Query, description = "是否返回完整信息(需要登录)"))
)]
pub async fn get_server_detail(
    State(db): State<DatabaseConnection>,
    Path(id): Path<u64>,
    Query(full_info): Query<Option<bool>>,
    user_claims: Option<Extension<Claims>>
) -> ApiResult<Json<ServerDetail>> {
    let user_id = user_claims.map(|Extension(claims)| claims.id);

    let full_info = full_info.unwrap_or(false);

    let result = ServerService::get_server_detail(&db, user_id, id, full_info).await?;

    Ok(Json(result))
}
