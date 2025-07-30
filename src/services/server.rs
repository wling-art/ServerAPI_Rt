use std::collections::HashMap;

use crate::entities::{files, server, server_stats};
use crate::{
    config::S3Config,
    entities::prelude::{
        Files, Gallery, GalleryImage as GalleryImageEntity, Server,
        ServerStats as ServerStatsEntity, UserServer, Users,
    },
    entities::{gallery, gallery_image, user_server},
    errors::ApiResult,
    handlers::servers::ListQuery,
    schemas::servers::{
        ApiAuthMode, ApiServerType, GalleryImage, GalleryImageSchema, ManagerInfo, Motd,
        ServerDetail, ServerGallery, ServerManagerRole, ServerManagersResponse, ServerStats,
        UpdateServerRequest,
    },
    services::{database::DatabaseConnection, file_upload::FileUploadService},
};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

use chrono::Utc;
use sea_orm::JsonValue;
use sea_orm::*;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use validator::Validate;

pub struct PaginatedServerResult {
    pub data: Vec<ServerDetail>,
    pub total: i64,
}

pub struct ServerService;

impl ServerService {
    pub async fn get_servers_with_filters(
        db: &DatabaseConnection,
        user_id: Option<i32>,
        list_query: &ListQuery,
    ) -> ApiResult<PaginatedServerResult> {
        let mut query = Server::find();

        if list_query.is_member {
            query = query.filter(server::Column::IsMember.eq(list_query.is_member));
        }

        if let Some(modes) = &list_query.r#type {
            query = query.filter(server::Column::Type.is_in(modes));
        }

        if let Some(auth_modes) = &list_query.auth_mode {
            query = query.filter(server::Column::AuthMode.is_in(auth_modes));
        }

        let mut servers = query
            .order_by_asc(server::Column::Id)
            .all(db.as_ref())
            .await?;

        if servers.is_empty() {
            return Ok(PaginatedServerResult {
                data: vec![],
                total: 0,
            });
        }

        if let Some(required_tags) = &list_query.tags {
            servers.retain(|server| Self::server_has_required_tags(&server.tags, required_tags));
        }

        let total = servers.len() as i64;

        let mut rng = if let Some(seed_val) = list_query.seed {
            StdRng::seed_from_u64(seed_val as u64)
        } else {
            StdRng::seed_from_u64(rand::random())
        };
        servers.shuffle(&mut rng);

        let start = ((list_query.page - 1) * list_query.page_size) as usize;
        let take = list_query.page_size as usize;

        if start >= servers.len() {
            return Ok(PaginatedServerResult {
                data: vec![],
                total,
            });
        }

        let page_servers: Vec<_> = servers.into_iter().skip(start).take(take).collect();
        let server_ids: Vec<i32> = page_servers.iter().map(|s| s.id).collect();

        if server_ids.is_empty() {
            return Ok(PaginatedServerResult {
                data: vec![],
                total,
            });
        }

        let (server_statses, user_servers, cover_files) = tokio::try_join!(
            ServerStatsEntity::find()
                .filter(server_stats::Column::ServerId.is_in(server_ids.clone()))
                .order_by_desc(server_stats::Column::Timestamp)
                .all(db.as_ref()),
            async {
                if let Some(uid) = user_id {
                    UserServer::find()
                        .filter(user_server::Column::UserId.eq(uid))
                        .filter(user_server::Column::ServerId.is_in(server_ids.clone()))
                        .all(db.as_ref())
                        .await
                } else {
                    Ok(vec![])
                }
            },
            async {
                let cover_hashes: Vec<String> = page_servers
                    .iter()
                    .filter_map(|s| s.cover_hash_id.as_ref())
                    .cloned()
                    .collect();

                if !cover_hashes.is_empty() {
                    Files::find()
                        .filter(files::Column::HashValue.is_in(cover_hashes))
                        .all(db.as_ref())
                        .await
                } else {
                    Ok(vec![])
                }
            }
        )?;

        let stats_map = Self::build_stats_map(&server_statses);
        let user_permissions = Self::build_user_permissions_map(&user_servers);
        let cover_file_map = Self::build_cover_file_map(&cover_files);

        let server_list = Self::convert_servers_to_details(
            page_servers,
            &stats_map,
            &user_permissions,
            &cover_file_map,
        )?;

        Ok(PaginatedServerResult {
            data: server_list,
            total,
        })
    }

    pub async fn get_server_detail(
        db: &DatabaseConnection,
        user_id: Option<i32>,
        server_id: i32,
        require_login: bool,
    ) -> ApiResult<ServerDetail> {
        if require_login && user_id.is_none() {
            return Err(crate::errors::ApiError::Unauthorized(
                "未登录，禁止访问".to_string(),
            ));
        }

        let server = Server::find_by_id(server_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| crate::errors::ApiError::NotFound("服务器不存在".to_string()))?;

        let (server_stats, user_server, cover_file) = tokio::try_join!(
            ServerStatsEntity::find()
                .filter(server_stats::Column::ServerId.eq(server.id))
                .order_by_desc(server_stats::Column::Timestamp)
                .one(db.as_ref()),
            async {
                if let Some(uid) = user_id {
                    UserServer::find()
                        .filter(user_server::Column::UserId.eq(uid))
                        .filter(user_server::Column::ServerId.eq(server.id))
                        .one(db.as_ref())
                        .await
                } else {
                    Ok(None)
                }
            },
            async {
                if let Some(ref cover_hash) = server.cover_hash_id {
                    Files::find()
                        .filter(files::Column::HashValue.eq(cover_hash))
                        .one(db.as_ref())
                        .await
                } else {
                    Ok(None)
                }
            }
        )?;

        let user_role = user_server.map(|us| us.role);
        if require_login && user_role.is_none() {
            return Err(crate::errors::ApiError::Unauthorized(
                "无权限访问该服务器".to_string(),
            ));
        }

        let stats = if let Some(stats_model) = server_stats {
            if let Some(ref stat_data) = stats_model.stat_data {
                Self::parse_server_stats(stat_data).ok()
            } else {
                None
            }
        } else {
            None
        };

        let cover_url = if let (Some(_hash), Some(file_model)) = (&server.cover_hash_id, cover_file)
        {
            Some(file_model.file_path)
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
            r#type: match server.r#type.as_str() {
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
            tags: Self::parse_server_tags(&server.tags),
            is_hide: server.is_hide,
            stats,
            permission: user_role.unwrap_or_else(|| "guest".to_string()),
            cover_url,
        })
    }

    fn build_stats_map(
        server_statses: &[server_stats::Model],
    ) -> HashMap<i32, &server_stats::Model> {
        let mut stats_map = HashMap::new();
        for stats in server_statses {
            stats_map.entry(stats.server_id).or_insert(stats);
        }
        stats_map
    }

    fn build_user_permissions_map(user_servers: &[user_server::Model]) -> HashMap<i32, String> {
        user_servers
            .iter()
            .map(|us| (us.server_id, us.role.clone()))
            .collect()
    }

    fn build_cover_file_map(cover_files: &[files::Model]) -> HashMap<String, String> {
        cover_files
            .iter()
            .map(|file_model| (file_model.hash_value.clone(), file_model.file_path.clone()))
            .collect()
    }

    fn server_has_required_tags(server_tags_json: &JsonValue, required_tags: &[String]) -> bool {
        if server_tags_json.is_null()
            || (server_tags_json.is_array() && server_tags_json.as_array().unwrap().is_empty())
        {
            return false;
        }

        match server_tags_json.as_array() {
            Some(server_tags) => {
                let server_tag_strings: Vec<String> = server_tags
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                required_tags
                    .iter()
                    .any(|required_tag| server_tag_strings.contains(required_tag))
            }
            None => false,
        }
    }

    fn convert_servers_to_details(
        servers: Vec<server::Model>,
        stats_map: &HashMap<i32, &server_stats::Model>,
        user_permissions: &HashMap<i32, String>,
        cover_file_map: &HashMap<String, String>,
    ) -> ApiResult<Vec<ServerDetail>> {
        let server_list = servers
            .into_iter()
            .map(|server| {
                let tags = Self::parse_server_tags(&server.tags);

                let server_type: ApiServerType =
                    server.r#type.parse().unwrap_or(ApiServerType::Java);
                let auth_mode: ApiAuthMode =
                    server.auth_mode.parse().unwrap_or(ApiAuthMode::Official);

                let stats = stats_map.get(&server.id).and_then(|stats_model| {
                    stats_model
                        .stat_data
                        .as_ref()
                        .and_then(|data| Self::parse_server_stats(data).ok())
                });

                let permission = user_permissions
                    .get(&server.id)
                    .cloned()
                    .unwrap_or_else(|| "guest".to_string());

                let cover_url = Self::build_cover_url(&server.cover_hash_id, cover_file_map);

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
                    stats,
                    permission,
                    cover_url,
                }
            })
            .collect();

        Ok(server_list)
    }

    fn parse_server_tags(tags_json: &JsonValue) -> Option<Vec<String>> {
        if tags_json.is_null() {
            return None;
        }

        tags_json.as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
    }

    fn build_cover_url(
        cover_hash: &Option<String>,
        cover_file_map: &HashMap<String, String>,
    ) -> Option<String> {
        cover_hash
            .as_ref()
            .and_then(|hash| cover_file_map.get(hash))
            .cloned()
    }

    fn build_image_url(file_path: &str) -> String {
        if file_path.starts_with("http://") || file_path.starts_with("https://") {
            file_path.to_string()
        } else {
            format!("/static/{file_path}")
        }
    }

    fn parse_server_stats(stat_data: &Value) -> ApiResult<ServerStats> {
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

        Ok(ServerStats {
            players,
            delay,
            version,
            motd,
            icon,
        })
    }

    pub async fn update_server_by_id(
        db: &DatabaseConnection,
        s3_config: &crate::config::S3Config,
        server_id: i32,
        update_data: UpdateServerRequest,
        current_user_id: i32,
    ) -> ApiResult<ServerDetail> {
        let server = Server::find_by_id(server_id)
            .one(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?
            .ok_or_else(|| crate::errors::ApiError::NotFound("未找到该服务器".to_string()))?;

        Self::check_server_edit_permission(db, server_id, current_user_id).await?;

        if update_data.name.trim().is_empty()
            && update_data.ip.trim().is_empty()
            && update_data.desc.trim().is_empty()
        {
            return Err(crate::errors::ApiError::BadRequest(
                "更新字段不能为空".to_string(),
            ));
        }

        update_data
            .validate()
            .map_err(|e| crate::errors::ApiError::BadRequest(format!("参数验证失败: {e}")))?;

        let original_cover_hash = server.cover_hash_id.clone();
        let cover_hash = if let Some(ref cover_data) = update_data.cover {
            let filename = cover_data
                .metadata
                .file_name
                .as_deref()
                .unwrap_or("cover.jpg");
            let file_model = FileUploadService::validate_and_upload_cover(
                db,
                s3_config,
                cover_data.contents.to_vec(),
                filename,
            )
            .await?;
            Some(file_model.hash_value)
        } else {
            original_cover_hash
        };

        let tags_json = serde_json::to_value(&update_data.tags)
            .map_err(|e| crate::errors::ApiError::Internal(format!("标签序列化失败: {e}")))?;

        let mut server_active: server::ActiveModel = server.into();
        server_active.name = Set(update_data.name.clone());
        server_active.ip = Set(update_data.ip.clone());
        server_active.desc = Set(update_data.desc.clone());
        server_active.tags = Set(tags_json);
        server_active.version = Set(update_data.version.clone());
        server_active.link = Set(update_data.link.clone());
        if let Some(hash) = cover_hash {
            server_active.cover_hash_id = Set(Some(hash));
        }

        let updated_server = server_active
            .update(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

        Self::get_server_detail(db, Some(current_user_id), updated_server.id, true).await
    }

    async fn check_server_edit_permission(
        db: &DatabaseConnection,
        server_id: i32,
        user_id: i32,
    ) -> ApiResult<()> {
        let user_server = UserServer::find()
            .filter(user_server::Column::UserId.eq(user_id))
            .filter(user_server::Column::ServerId.eq(server_id))
            .one(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

        match user_server {
            Some(us) => {
                if us.role == "admin" || us.role == "owner" {
                    Ok(())
                } else {
                    Err(crate::errors::ApiError::Authorization(
                        "无权限编辑该服务器".to_string(),
                    ))
                }
            }
            None => Err(crate::errors::ApiError::Authorization(
                "无权限编辑该服务器".to_string(),
            )),
        }
    }

    pub async fn get_server_gallery(
        db: &DatabaseConnection,
        server_id: i32,
    ) -> ApiResult<ServerGallery> {
        if server_id <= 0 {
            return Err(crate::errors::ApiError::BadRequest(
                "服务器ID必须大于0".to_string(),
            ));
        }

        let server = Server::find_by_id(server_id)
            .one(db.as_ref())
            .await
            .map_err(|e| {
                tracing::error!("查询服务器失败: server_id={}, error={}", server_id, e);
                crate::errors::ApiError::Database(format!("查询服务器失败: {e}"))
            })?
            .ok_or_else(|| {
                tracing::warn!("服务器不存在: server_id={}", server_id);
                crate::errors::ApiError::NotFound("服务器不存在".to_string())
            })?;

        let gallery_images = Self::get_server_gallery_images(db, &server).await?;

        tracing::info!(
            "成功获取服务器相册: server_id={}, gallery_count={}",
            server_id,
            gallery_images.len()
        );

        Ok(ServerGallery {
            id: server.id,
            name: server.name,
            gallery_images,
        })
    }

    async fn get_server_gallery_images(
        db: &DatabaseConnection,
        server: &server::Model,
    ) -> ApiResult<Vec<GalleryImage>> {
        let gallery_id = match server.gallery_id {
            Some(id) => {
                tracing::debug!(
                    "服务器关联相册ID: server_id={}, gallery_id={}",
                    server.id,
                    id
                );
                id
            }
            None => {
                tracing::debug!("服务器未关联相册: server_id={}", server.id);
                return Ok(vec![]);
            }
        };

        let gallery_images = GalleryImageEntity::find()
            .filter(gallery_image::Column::GalleryId.eq(gallery_id))
            .all(db.as_ref())
            .await
            .map_err(|e| {
                tracing::error!("查询相册图片失败: gallery_id={}, error={}", gallery_id, e);
                crate::errors::ApiError::Database(format!("查询相册图片失败: {e}"))
            })?;

        if gallery_images.is_empty() {
            tracing::debug!("相册无图片: gallery_id={}", gallery_id);
            return Ok(vec![]);
        }

        tracing::debug!(
            "找到相册图片数量: gallery_id={}, count={}",
            gallery_id,
            gallery_images.len()
        );

        let image_hashes: Vec<String> = gallery_images
            .iter()
            .map(|img| img.image_hash_id.clone())
            .collect();

        let image_files = Files::find()
            .filter(files::Column::HashValue.is_in(image_hashes.clone()))
            .all(db.as_ref())
            .await
            .map_err(|e| {
                tracing::error!("查询图片文件失败: hashes={:?}, error={}", image_hashes, e);
                crate::errors::ApiError::Database(format!("查询图片文件失败: {e}"))
            })?;

        let file_map: HashMap<String, String> = image_files
            .iter()
            .map(|file_model| (file_model.hash_value.clone(), file_model.file_path.clone()))
            .collect();

        let mut gallery_list = Vec::new();
        let mut missing_files = Vec::new();

        for gallery_image in gallery_images {
            if let Some(file_path) = file_map.get(&gallery_image.image_hash_id) {
                let image_url = Self::build_image_url(file_path);
                gallery_list.push(GalleryImage {
                    id: gallery_image.id,
                    title: gallery_image.title,
                    description: gallery_image.description,
                    image_url,
                });
            } else {
                missing_files.push(gallery_image.image_hash_id.clone());
            }
        }

        if !missing_files.is_empty() {
            tracing::warn!(
                "部分图片文件缺失: gallery_id={}, missing_files={:?}",
                gallery_id,
                missing_files
            );
        }

        tracing::debug!(
            "成功构建相册列表: gallery_id={}, valid_images={}",
            gallery_id,
            gallery_list.len()
        );

        Ok(gallery_list)
    }

    pub async fn get_server_managers(
        db: &DatabaseConnection,
        server_id: i32,
    ) -> ApiResult<ServerManagersResponse> {
        let _server = Server::find_by_id(server_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| crate::errors::ApiError::NotFound("服务器不存在".to_string()))?;

        let managers = UserServer::find()
            .filter(user_server::Column::ServerId.eq(server_id))
            .find_also_related(Users)
            .all(db.as_ref())
            .await?;

        let avatar_hashes: Vec<String> = managers
            .iter()
            .filter_map(|(_, user_opt)| {
                user_opt
                    .as_ref()
                    .and_then(|user| user.avatar_hash_id.clone())
            })
            .collect();

        let avatar_files = if !avatar_hashes.is_empty() {
            Files::find()
                .filter(files::Column::HashValue.is_in(avatar_hashes))
                .all(db.as_ref())
                .await?
        } else {
            vec![]
        };

        let avatar_file_map: HashMap<String, String> = avatar_files
            .iter()
            .map(|file_model| (file_model.hash_value.clone(), file_model.file_path.clone()))
            .collect();

        let mut owners = Vec::new();
        let mut admins = Vec::new();

        for (user_server_relation, user_opt) in managers {
            if let Some(user) = user_opt {
                let avatar_url = if let Some(avatar_hash_id) = &user.avatar_hash_id {
                    let file_path = avatar_file_map.get(avatar_hash_id).ok_or_else(|| {
                        crate::errors::ApiError::Internal(format!(
                            "头像文件不存在: {avatar_hash_id}"
                        ))
                    })?;

                    file_path.clone()
                } else {
                    return Err(crate::errors::ApiError::Internal(
                        "用户缺少头像信息".to_string(),
                    ));
                };

                let role = match user_server_relation.role.as_str() {
                    "owner" => ServerManagerRole::Owner,
                    "admin" => ServerManagerRole::Admin,
                    _ => continue,
                };

                let manager_info = ManagerInfo {
                    id: user.id,
                    display_name: user.display_name,
                    is_active: user.is_active,
                    avatar_url,
                };

                match role {
                    ServerManagerRole::Owner => owners.push(manager_info),
                    ServerManagerRole::Admin => admins.push(manager_info),
                }
            }
        }

        Ok(ServerManagersResponse { owners, admins })
    }

    pub async fn has_server_edit_permission(
        db: &DatabaseConnection,
        user_id: i32,
        server_id: i32,
    ) -> ApiResult<bool> {
        let user_server = UserServer::find()
            .filter(user_server::Column::UserId.eq(user_id))
            .filter(user_server::Column::ServerId.eq(server_id))
            .filter(user_server::Column::Role.is_in(["owner", "admin"]))
            .one(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

        Ok(user_server.is_some())
    }

    pub async fn add_gallery_image(
        db: &DatabaseConnection,
        s3_config: &S3Config,
        server_id: i32,
        gallery_data: &GalleryImageSchema,
    ) -> ApiResult<()> {
        let server = Server::find_by_id(server_id)
            .one(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?
            .ok_or_else(|| crate::errors::ApiError::NotFound("服务器不存在".to_string()))?;

        gallery_data
            .validate()
            .map_err(|e| crate::errors::ApiError::BadRequest(format!("参数验证失败: {e}")))?;

        let gallery_id = if let Some(gallery_id) = server.gallery_id {
            gallery_id
        } else {
            let new_gallery = gallery::ActiveModel {
                created_at: Set(Utc::now()),
                ..Default::default()
            };
            let gallery = Gallery::insert(new_gallery)
                .exec_with_returning(db.as_ref())
                .await
                .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

            let mut server_active: server::ActiveModel = server.into();
            server_active.gallery_id = Set(Some(gallery.id));
            Server::update(server_active)
                .exec(db.as_ref())
                .await
                .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

            gallery.id
        };

        let image_content = gallery_data.image.contents.to_vec();
        let filename = gallery_data
            .image
            .metadata
            .file_name
            .as_deref()
            .unwrap_or("image.jpg");

        let image_file =
            FileUploadService::validate_and_upload_gallery(db, s3_config, image_content, filename)
                .await?;

        let gallery_image = gallery_image::ActiveModel {
            gallery_id: Set(gallery_id),
            title: Set(gallery_data.title.clone()),
            description: Set(gallery_data.description.clone()),
            image_hash_id: Set(image_file.hash_value),
            ..Default::default()
        };

        GalleryImageEntity::insert(gallery_image)
            .exec(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_gallery_image(
        db: &DatabaseConnection,
        s3_config: &S3Config,
        server_id: i32,
        image_id: i32,
    ) -> ApiResult<()> {
        use crate::services::file_upload::FileUploadService;

        let server = Server::find_by_id(server_id)
            .one(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?
            .ok_or_else(|| crate::errors::ApiError::NotFound("服务器不存在".to_string()))?;

        let gallery_id = server
            .gallery_id
            .ok_or_else(|| crate::errors::ApiError::NotFound("该服务器没有画册".to_string()))?;

        let gallery_image = GalleryImageEntity::find_by_id(image_id)
            .one(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?
            .ok_or_else(|| crate::errors::ApiError::NotFound("图片不存在".to_string()))?;

        if gallery_image.gallery_id != gallery_id {
            return Err(crate::errors::ApiError::Forbidden(
                "图片不属于该服务器".to_string(),
            ));
        }

        FileUploadService::delete_file(s3_config, &gallery_image.image_hash_id).await?;

        Files::delete_by_id(&gallery_image.image_hash_id)
            .exec(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

        GalleryImageEntity::delete_by_id(image_id)
            .exec(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn total_players(
        db: &DatabaseConnection,
    ) -> ApiResult<crate::schemas::servers::ServerTotalPlayers> {
        let server_statses = ServerStatsEntity::find()
            .select_only()
            .column(server_stats::Column::StatData)
            .all(db.as_ref())
            .await
            .map_err(|e| crate::errors::ApiError::Database(e.to_string()))?;

        let mut total_players = 0i32;

        for server_stats in server_statses {
            if let Some(stat_data) = &server_stats.stat_data {
                if let Some(players_obj) = stat_data.get("players") {
                    if let Some(online_players) = players_obj.get("online") {
                        if let Some(online_count) = online_players.as_i64() {
                            total_players += online_count as i32;
                        }
                    }
                }
            }
        }

        Ok(crate::schemas::servers::ServerTotalPlayers { total_players })
    }
}
