use std::collections::HashMap;

use sea_orm::*;
use serde_json::Value;

use crate::{
    entities::{
        file,
        server,
        server_status,
        user_server,
        FileEntity,
        ServerEntity,
        ServerStatusEntity,
        UserServerEntity,
    },
    errors::ApiResult,
    handlers::servers::ListQuery,
    schemas::servers::{ ApiAuthMode, ApiServerType, Motd, ServerDetail, ServerStatus },
    services::database::DatabaseConnection,
};

/// 分页结果结构体
pub struct PaginatedServerResult {
    /// 当前页的服务器列表
    pub data: Vec<ServerDetail>,
    /// 过滤后的服务器总数
    pub total: i64,
}

pub struct ServerService;

impl ServerService {
    /// 获取带过滤条件的服务器列表 - 优化版本
    pub async fn get_servers_with_filters(
        db: &DatabaseConnection,
        user_id: Option<i32>,
        list_query: &ListQuery
    ) -> ApiResult<PaginatedServerResult> {
        // 构建查询，应用过滤条件
        let mut query = ServerEntity::find();

        // 应用成员过滤
        if list_query.is_member {
            query = query.filter(server::Column::IsMember.eq(list_query.is_member));
        }

        // 应用服务器类型过滤
        if let Some(modes) = &list_query.r#type {
            query = query.filter(server::Column::ServerType.is_in(modes));
        }

        // 应用认证模式过滤
        if let Some(auth_modes) = &list_query.auth_mode {
            query = query.filter(server::Column::AuthMode.is_in(auth_modes));
        }

        // 获取所有符合条件的服务器
        let mut servers = query.order_by_asc(server::Column::Id).all(db.as_ref()).await?;

        if servers.is_empty() {
            return Ok(PaginatedServerResult {
                data: vec![],
                total: 0,
            });
        }

        // 应用标签过滤（在应用层面，因为 JSON 字段难以在数据库层优化）
        if let Some(required_tags) = &list_query.tags {
            servers.retain(|server| Self::server_has_required_tags(&server.tags, &required_tags));
        }

        // 记录过滤后的总数
        let total = servers.len() as i64;

        // 随机排序）
        use rand::{ rngs::StdRng, seq::SliceRandom, SeedableRng };
        let mut rng = if let Some(seed_val) = list_query.seed {
            StdRng::seed_from_u64(seed_val as u64)
        } else {
            StdRng::from_entropy()
        };
        servers.shuffle(&mut rng);

        // 应用分页 - 只处理需要的数据
        let start = ((list_query.page - 1) * list_query.page_size) as usize;
        let take = list_query.page_size as usize;

        if start >= servers.len() {
            return Ok(PaginatedServerResult {
                data: vec![],
                total,
            });
        }

        let page_servers: Vec<_> = servers.into_iter().skip(start).take(take).collect();
        let server_ids: Vec<i32> = page_servers
            .iter()
            .map(|s| s.id)
            .collect();

        if server_ids.is_empty() {
            return Ok(PaginatedServerResult {
                data: vec![],
                total,
            });
        }

        // 并行执行多个数据库查询以提高性能
        let (server_statuses, user_servers, cover_files) = tokio::try_join!(
            // 查询服务器状态
            ServerStatusEntity::find()
                .filter(server_status::Column::ServerId.is_in(server_ids.clone()))
                .order_by_desc(server_status::Column::Timestamp)
                .all(db.as_ref()),
            // 查询用户权限
            async {
                if let Some(uid) = user_id {
                    UserServerEntity::find()
                        .filter(user_server::Column::UserId.eq(uid))
                        .filter(user_server::Column::ServerId.is_in(server_ids.clone()))
                        .all(db.as_ref()).await
                } else {
                    Ok(vec![])
                }
            },
            // 查询封面文件
            async {
                let cover_hashes: Vec<String> = page_servers
                    .iter()
                    .filter_map(|s| s.cover_hash.as_ref())
                    .cloned()
                    .collect();

                if !cover_hashes.is_empty() {
                    FileEntity::find()
                        .filter(file::Column::HashValue.is_in(cover_hashes))
                        .all(db.as_ref()).await
                } else {
                    Ok(vec![])
                }
            }
        )?;

        // 构建高效的映射表
        let status_map = Self::build_status_map(&server_statuses);
        let user_permissions = Self::build_user_permissions_map(&user_servers);
        let cover_file_map = Self::build_cover_file_map(&cover_files);

        // 转换为 ServerDetail
        let server_list = Self::convert_servers_to_details(
            page_servers,
            &status_map,
            &user_permissions,
            &cover_file_map
        )?;

        Ok(PaginatedServerResult {
            data: server_list,
            total,
        })
    }

    /// 获取单个服务器的详细信息
    pub async fn get_server_detail(
        db: &DatabaseConnection,
        user_id: Option<i32>,
        server_id: u64,
        require_login: bool
    ) -> ApiResult<ServerDetail> {
        // 如果强制要求登录但未提供 user_id，直接返回未登录错误
        if require_login && user_id.is_none() {
            return Err(crate::errors::ApiError::Unauthorized("未登录，禁止访问".to_string()));
        }

        // 查询服务器基本信息
        let server = ServerEntity::find_by_id(server_id as i32)
            .one(db.as_ref()).await?
            .ok_or_else(|| crate::errors::ApiError::NotFound("服务器不存在".to_string()))?;

        // 并行执行多个数据库查询
        let (server_status, user_server, cover_file) = tokio::try_join!(
            // 查询最新的服务器状态
            ServerStatusEntity::find()
                .filter(server_status::Column::ServerId.eq(server.id))
                .order_by_desc(server_status::Column::Timestamp)
                .one(db.as_ref()),
            // 查询用户权限
            async {
                if let Some(uid) = user_id {
                    UserServerEntity::find()
                        .filter(user_server::Column::UserId.eq(uid))
                        .filter(user_server::Column::ServerId.eq(server.id))
                        .one(db.as_ref()).await
                } else {
                    Ok(None)
                }
            },
            // 查询封面文件
            async {
                if let Some(ref cover_hash) = server.cover_hash {
                    FileEntity::find()
                        .filter(file::Column::HashValue.eq(cover_hash))
                        .one(db.as_ref()).await
                } else {
                    Ok(None)
                }
            }
        )?;

        // user_role 处理
        let user_role = user_server.map(|us| us.role);
        // 如果强制要求登录但 user_role 仍为 None，说明用户无权限
        if require_login && user_role.is_none() {
            return Err(crate::errors::ApiError::Unauthorized("无权限访问该服务器".to_string()));
        }

        // 转换为 ServerDetail
        let status = if let Some(status_model) = server_status {
            if let Some(ref stat_data) = status_model.stat_data {
                match Self::parse_server_status(stat_data) {
                    Ok(parsed_status) => Some(parsed_status),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let cover_url = if let (Some(_hash), Some(file_model)) = (&server.cover_hash, cover_file) {
            if
                file_model.file_path.starts_with("http://") ||
                file_model.file_path.starts_with("https://")
            {
                Some(file_model.file_path)
            } else {
                Some(format!("/static/{}", file_model.file_path))
            }
        } else {
            None
        };

        Ok(ServerDetail {
            id: server.id,
            name: server.name,
            ip: if server.is_hide {
                None
            } else {
                Some(server.ip)
            },
            r#type: match server.server_type.as_str() {
                "JAVA" => ApiServerType::Java,
                "BEDROCK" => ApiServerType::Bedrock,
                _ => ApiServerType::Java,
            },
            version: server.version,
            desc: server.desc,
            link: server.link,
            is_member: server.is_member,
            auth_mode: match server.auth_mode.as_str() {
                "OFFLINE" => ApiAuthMode::Offline,
                "OFFICIAL" => ApiAuthMode::Official,
                "YGGDRASIL" => ApiAuthMode::Yggdrasil,
                _ => ApiAuthMode::Official,
            },
            tags: serde_json::from_str(&server.tags).unwrap_or_default(),
            is_hide: server.is_hide,
            status,
            permission: user_role.unwrap_or_else(|| "guest".to_string()),
            cover_url,
        })
    }

    /// 构建服务器状态映射表
    fn build_status_map(
        server_statuses: &[server_status::Model]
    ) -> HashMap<i32, &server_status::Model> {
        let mut status_map = HashMap::new();
        for status in server_statuses {
            // 由于已经按时间倒序排列，第一个遇到的就是最新的
            status_map.entry(status.server_id).or_insert(status);
        }
        status_map
    }

    /// 构建用户权限映射表
    fn build_user_permissions_map(user_servers: &[user_server::Model]) -> HashMap<i32, String> {
        user_servers
            .iter()
            .map(|us| (us.server_id, us.role.clone()))
            .collect()
    }

    /// 构建封面文件映射表
    fn build_cover_file_map(cover_files: &[file::Model]) -> HashMap<String, String> {
        cover_files
            .iter()
            .map(|file_model| (file_model.hash_value.clone(), file_model.file_path.clone()))
            .collect()
    }

    /// 检查服务器是否包含所需标签
    fn server_has_required_tags(server_tags_json: &str, required_tags: &[String]) -> bool {
        if server_tags_json.is_empty() {
            return false;
        }

        match serde_json::from_str::<Value>(server_tags_json) {
            Ok(json_value) =>
                match json_value.as_array() {
                    Some(server_tags) => {
                        let server_tag_strings: Vec<String> = server_tags
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        // 检查是否包含所需的任何一个标签
                        required_tags
                            .iter()
                            .any(|required_tag| server_tag_strings.contains(required_tag))
                    }
                    None => false,
                }
            Err(_) => false,
        }
    }

    /// 将服务器模型转换为 ServerDetail
    fn convert_servers_to_details(
        servers: Vec<server::Model>,
        status_map: &HashMap<i32, &server_status::Model>,
        user_permissions: &HashMap<i32, String>,
        cover_file_map: &HashMap<String, String>
    ) -> ApiResult<Vec<ServerDetail>> {
        let server_list = servers
            .into_iter()
            .map(|server| {
                // 解析 tags
                let tags = Self::parse_server_tags(&server.tags);

                // 转换类型
                let server_type = ApiServerType::from_str(&server.server_type).unwrap_or(
                    ApiServerType::Java
                );
                let auth_mode = ApiAuthMode::from_str(&server.auth_mode).unwrap_or(
                    ApiAuthMode::Official
                );

                // 处理服务器状态
                let status = status_map
                    .get(&server.id)
                    .and_then(|status_model| {
                        status_model.stat_data
                            .as_ref()
                            .and_then(|data| Self::parse_server_status(data).ok())
                    });

                // 获取用户权限
                let permission = user_permissions
                    .get(&server.id)
                    .cloned()
                    .unwrap_or_else(|| "guest".to_string());

                // 生成封面URL
                let cover_url = Self::build_cover_url(&server.cover_hash, cover_file_map);

                ServerDetail {
                    id: server.id,
                    name: server.name,
                    ip: if server.is_hide {
                        None
                    } else {
                        Some(server.ip)
                    },
                    r#type: server_type,
                    version: server.version,
                    desc: server.desc,
                    link: server.link,
                    is_member: server.is_member,
                    auth_mode,
                    tags,
                    is_hide: server.is_hide,
                    status,
                    permission,
                    cover_url,
                }
            })
            .collect();

        Ok(server_list)
    }

    /// 解析服务器标签
    fn parse_server_tags(tags_json: &str) -> Option<Vec<String>> {
        if tags_json.is_empty() {
            return None;
        }

        match serde_json::from_str::<Value>(tags_json) {
            Ok(json_value) =>
                match json_value.as_array() {
                    Some(arr) =>
                        Some(
                            arr
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        ),
                    None => None,
                }
            Err(_) => None,
        }
    }

    /// 构建封面URL
    fn build_cover_url(
        cover_hash: &Option<String>,
        cover_file_map: &HashMap<String, String>
    ) -> Option<String> {
        cover_hash
            .as_ref()
            .and_then(|hash| cover_file_map.get(hash))
            .map(|file_path| {
                if file_path.starts_with("http://") || file_path.starts_with("https://") {
                    file_path.clone()
                } else {
                    format!("/static/{}", file_path)
                }
            })
    }

    /// 解析服务器状态JSON为ServerStatus结构
    fn parse_server_status(stat_data: &Value) -> ApiResult<ServerStatus> {
        let players = stat_data
            .get("players")
            .and_then(|p| p.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.as_i64().unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();

        let delay = stat_data
            .get("delay")
            .and_then(|d| d.as_f64())
            .unwrap_or(0.0);

        let version = stat_data
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let motd = stat_data
            .get("motd")
            .and_then(|m| m.as_object())
            .map(|motd_obj| Motd {
                plain: motd_obj
                    .get("plain")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string(),
                html: motd_obj
                    .get("html")
                    .and_then(|h| h.as_str())
                    .unwrap_or("")
                    .to_string(),
                minecraft: motd_obj
                    .get("minecraft")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
                ansi: motd_obj
                    .get("ansi")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
            .unwrap_or_default();

        let icon = stat_data
            .get("icon")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());

        Ok(ServerStatus {
            players,
            delay,
            version,
            motd,
            icon,
        })
    }
}
