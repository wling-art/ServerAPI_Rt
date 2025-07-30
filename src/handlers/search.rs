use axum::{
    extract::{Query},
    Json,
};
use crate::{
    errors::ApiResult,
    schemas::search::{SearchParams, SearchResponse},
    services::search::client::MeilisearchClient,
};

#[utoipa::path(
    get,
    summary = "搜索服务器",
    path = "/v2/search",
    tag = "search",
    responses(
        (status = 200, description = "搜索结果", body = SearchResponse),
    ),
    params(
        SearchParams
    )
)]
pub async fn search_server(Query(params): Query<SearchParams>) -> ApiResult<Json<SearchResponse>> {
    // 构建搜索查询
    let results = MeilisearchClient::search_servers(Query(params)).await?;

    Ok(Json(results))
}
