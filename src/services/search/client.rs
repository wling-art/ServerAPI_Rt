use crate::entities::server::Entity as Server;
use crate::schemas::search::{SearchFilters, SearchParams, SearchResponse, ServerResult};
use crate::schemas::servers::{ApiAuthMode, ApiServerType};
use anyhow::Result;
use axum::extract::Query as AxumQuery;
use meilisearch_sdk::client::*;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tokio::time::{sleep, Duration};

/// Meilisearch 客户端
/// 用于与 Meilisearch 进行交互
#[derive(Debug)]
pub struct MeilisearchClient {
    client: Arc<Client>,
}

static MEILISEARCH_INSTANCE: OnceCell<Arc<MeilisearchClient>> = OnceCell::const_new();

impl SearchFilters {
    /// 将结构化过滤器转换为 Meilisearch 过滤字符串
    pub fn to_filter_string(&self) -> String {
        let mut filters = Vec::new();

        // 服务器类型过滤
        if let Some(types) = &self.server_type {
            if !types.is_empty() {
                let type_filters: Vec<String> = types
                    .iter()
                    .map(|t| {
                        format!(
                            "type = '{}'",
                            serde_json::to_string(t)
                                .unwrap_or_default()
                                .trim_matches('"')
                        )
                    })
                    .collect();
                filters.push(format!("({})", type_filters.join(" OR ")));
            }
        }

        // 标签过滤
        if let Some(tags) = &self.tags {
            if !tags.is_empty() {
                let tag_filters: Vec<String> =
                    tags.iter().map(|tag| format!("tags = '{}'", tag)).collect();
                filters.push(format!("({})", tag_filters.join(" OR ")));
            }
        }

        // 认证模式过滤
        if let Some(auth_modes) = &self.auth_mode {
            if !auth_modes.is_empty() {
                let auth_filters: Vec<String> = auth_modes
                    .iter()
                    .map(|mode| {
                        format!(
                            "auth_mode = '{}'",
                            serde_json::to_string(mode)
                                .unwrap_or_default()
                                .trim_matches('"')
                        )
                    })
                    .collect();
                filters.push(format!("({})", auth_filters.join(" OR ")));
            }
        }

        // 布尔值过滤
        if let Some(is_member) = self.is_member {
            filters.push(format!("is_member = {}", is_member));
        }

        if let Some(is_hide) = self.is_hide {
            filters.push(format!("is_hide = {}", is_hide));
        }

        // 版本过滤
        if let Some(versions) = &self.version {
            if !versions.is_empty() {
                let version_filters: Vec<String> = versions
                    .iter()
                    .map(|version| format!("version = '{}'", version))
                    .collect();
                filters.push(format!("({})", version_filters.join(" OR ")));
            }
        }

        filters.join(" AND ")
    }
}

impl SearchParams {
    /// 解析搜索参数，构建结构化过滤器
    pub fn parse_filters(&self) -> Result<SearchFilters> {
        let mut filters = SearchFilters::default();

        // 快捷过滤参数覆盖 JSON 字段
        if let Some(server_type) = &self.server_type {
            let parsed_type = match server_type {
                ApiServerType::Java => ApiServerType::Java,
                ApiServerType::Bedrock => ApiServerType::Bedrock,
            };
            filters.server_type = Some(vec![parsed_type]);
        }

        if let Some(tags_str) = &self.tags {
            let tags: Vec<String> = tags_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !tags.is_empty() {
                filters.tags = Some(tags);
            }
        }

        if let Some(auth_mode) = &self.auth_mode {
            let parsed_mode = match auth_mode {
                ApiAuthMode::Official => ApiAuthMode::Official,
                ApiAuthMode::Offline => ApiAuthMode::Offline,
                ApiAuthMode::Yggdrasil => ApiAuthMode::Yggdrasil,
            };
            filters.auth_mode = Some(vec![parsed_mode]);
        }

        if let Some(is_member) = self.is_member {
            filters.is_member = Some(is_member);
        }

        Ok(filters)
    }
}

impl MeilisearchClient {
    /// 初始化 Meilisearch 客户端
    pub async fn init(url: String, api_key: String) -> Result<()> {
        let client = Client::new(url, Some(api_key))
            .map_err(|e| anyhow::anyhow!("创建 Meilisearch 客户端失败: {}", e))?;

        let meili_client = Arc::new(MeilisearchClient {
            client: Arc::new(client),
        });

        MEILISEARCH_INSTANCE
            .set(meili_client.clone())
            .map_err(|_| anyhow::anyhow!("设置 Meilisearch 实例失败"))?;

        meili_client.init_meilisearch_index().await?;
        tracing::info!("Meilisearch 客户端初始化完成");
        Ok(())
    }

    /// 获取全局实例
    pub fn instance() -> Result<Arc<MeilisearchClient>> {
        MEILISEARCH_INSTANCE
            .get()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Meilisearch 客户端未初始化"))
    }

    /// 同步服务器数据到搜索索引
    pub async fn sync_server_search(&self, db: &DatabaseConnection) -> Result<()> {
        let servers = Server::find()
            .all(db)
            .await
            .map_err(|e| anyhow::anyhow!("查询服务器数据失败: {}", e))?;

        let documents: Vec<_> = servers
            .iter()
            .map(|server| {
                serde_json::json!({
                    "id": server.id,
                    "name": server.name,
                    "type": server.r#type,
                    "version": server.version,
                    "desc": server.desc,
                    "link": server.link,
                    "ip": server.ip,
                    "is_member": server.is_member,
                    "is_hide": server.is_hide,
                    "auth_mode": server.auth_mode,
                    "tags": server.tags,
                })
            })
            .collect();

        self.client
            .index("servers")
            .add_documents(&documents, Some("id"))
            .await
            .map_err(|e| anyhow::anyhow!("同步搜索索引失败: {}", e))?;

        tracing::info!("已同步 {} 条服务器记录到 Meilisearch 索引", documents.len());
        Ok(())
    }

    /// 定期同步搜索索引
    pub async fn sync_meilisearch_loop(
        &self,
        db: &DatabaseConnection,
        interval_secs: u64,
    ) -> Result<()> {
        tracing::info!("开始定期同步搜索索引，间隔: {} 秒", interval_secs);
        loop {
            match self.sync_server_search(db).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("同步搜索索引失败: {}", e);
                }
            }
            sleep(Duration::from_secs(interval_secs)).await;
        }
    }

    /// 初始化 Meilisearch 索引并设置相关配置
    pub async fn init_meilisearch_index(&self) -> Result<()> {
        let index = self.client.index("servers");

        // 可搜索字段
        index
            .set_searchable_attributes(["name", "desc", "ip", "tags", "type", "version"])
            .await
            .map_err(|e| anyhow::anyhow!("设置可搜索字段失败: {}", e))?;

        // 可过滤字段
        index
            .set_filterable_attributes([
                "type",
                "tags",
                "auth_mode",
                "is_member",
                "is_hide",
                "version",
            ])
            .await
            .map_err(|e| anyhow::anyhow!("设置可过滤字段失败: {}", e))?;

        // 设置排序字段
        index
            .set_sortable_attributes(["id", "name", "is_member"])
            .await
            .map_err(|e| anyhow::anyhow!("设置排序字段失败: {}", e))?;

        tracing::info!("Meilisearch 索引配置完成");
        Ok(())
    }

    /// 搜索服务器
    pub async fn search_servers(
        AxumQuery(params): AxumQuery<SearchParams>,
    ) -> Result<SearchResponse> {
        let start_time = std::time::Instant::now();
        let client = Self::instance()?;
        let index = client.client.index("servers");

        // 解析过滤器
        let filters = params.parse_filters()?;
        let filter_string = filters.to_filter_string();

        // 构建搜索请求
        let mut search_request = index.search();

        // 查询词
        if let Some(query) = &params.query {
            if !query.trim().is_empty() {
                search_request.with_query(query);
            }
        }

        // 设置分页
        let limit = params.limit.unwrap_or(10).min(100) as usize; // 限制最大返回数量
        let offset = params.offset.unwrap_or(0) as usize;
        search_request.with_limit(limit).with_offset(offset);

        // 设置过滤器
        if !filter_string.is_empty() {
            search_request.with_filter(&filter_string);
        }

        // 设置排序
        let sort_criteria: Vec<&str> = match params.sort.as_deref().unwrap_or_default() {
            "name_asc" => vec!["name:asc"],
            "name_desc" => vec!["name:desc"],
            "member_first" => vec!["is_member:desc", "name:asc"],
            _ => vec![],
        };
        if !sort_criteria.is_empty() {
            search_request.with_sort(&sort_criteria);
        }

        // 执行搜索
        let results = search_request
            .execute::<ServerResult>()
            .await
            .map_err(|e| anyhow::anyhow!("搜索执行失败: {}", e))?;

        let processing_time = start_time.elapsed().as_millis();

        Ok(SearchResponse {
            hits: results.hits.into_iter().map(|h| h.result).collect(),
            total: results.estimated_total_hits.unwrap_or(0),
            limit,
            offset,
            processing_time_ms: processing_time,
        })
    }

    /// 获取搜索统计信息
    pub async fn get_search_stats(&self) -> Result<String> {
        let index = self.client.index("servers");
        let stats = index
            .get_stats()
            .await
            .map_err(|e| anyhow::anyhow!("获取索引统计失败: {}", e))?;

        let stats_json = serde_json::json!({
            "number_of_documents": stats.number_of_documents,
            "is_indexing": stats.is_indexing,
            "field_distribution": stats.field_distribution
        });

        Ok(stats_json.to_string())
    }

    /// 清空索引
    pub async fn clear_index(&self) -> Result<()> {
        let index = self.client.index("servers");
        index
            .delete_all_documents()
            .await
            .map_err(|e| anyhow::anyhow!("清空索引失败: {}", e))?;
        tracing::info!("已清空搜索索引");
        Ok(())
    }
}
