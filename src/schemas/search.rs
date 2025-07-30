use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::schemas::servers::{ApiAuthMode, ApiServerType};

/// 结构化的搜索过滤器
#[derive(Debug, Clone, Deserialize, Serialize, Default, ToSchema)]
pub struct SearchFilters {
    /// 服务器类型过滤
    #[serde(rename = "type")]
    #[schema(example = "Java")]
    pub server_type: Option<Vec<ApiServerType>>,
    /// 标签过滤
    #[schema(example = "公益,生电,纯净")]
    pub tags: Option<Vec<String>>,
    /// 认证模式过滤
    #[schema(example = "Offline")]
    pub auth_mode: Option<Vec<ApiAuthMode>>,
    /// 是否为成员服务器
    #[schema(example = false)]
    pub is_member: Option<bool>,
    /// 是否隐藏 IP
    #[schema(example = false)]
    pub is_hide: Option<bool>,
    /// 版本过滤
    #[schema(example = "1.20.1,1.19.4")]
    pub version: Option<Vec<String>>,
}

/// 搜索参数
#[derive(Deserialize, IntoParams, ToSchema)]
pub struct SearchParams {
    /// 搜索关键词
    #[schema(example = "生存服务器")]
    pub query: Option<String>,
    /// 返回结果数量限制
    #[schema(example = 10)]
    pub limit: Option<u32>,
    /// 偏移量，用于分页
    #[schema(example = 0)]
    pub offset: Option<u32>,
    /// 服务器类型快捷过滤（与 SearchFilters 区分，单值）
    #[serde(rename = "type")]
    #[schema(example = "Java")]
    pub server_type: Option<ApiServerType>,
    /// 标签快捷过滤（逗号分隔，与 SearchFilters 区分，单字符串）
    #[schema(example = "公益,生电,纯净")]
    pub tags: Option<String>,
    /// 认证模式快捷过滤（与 SearchFilters 区分，单值）
    #[schema(example = "Offline")]
    pub auth_mode: Option<ApiAuthMode>,
    /// 是否会员服务器快捷过滤
    #[schema(example = false)]
    pub is_member: Option<bool>,
    /// 排序字段
    #[schema(example = "auth_mode")]
    pub sort: Option<String>,
}

/// 搜索结果
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ServerResult {
    /// 服务器 ID，服务器的唯一标识符
    #[schema(example = 1)]
    pub id: i32,
    /// 服务器名称，服务器的名称
    #[schema(example = "我的世界服务器")]
    pub name: String,
    /// 服务器 IP，服务器的 IP 地址，若隐藏则为 None
    #[schema(example = "mc.example.com:25565")]
    pub ip: Option<String>,
    /// 服务器类型，服务器所属的类型
    #[schema(example = "JAVA")]
    pub r#type: ApiServerType,
    /// 服务器版本，服务器运行的版本
    #[schema(example = "1.20.1")]
    pub version: String,
    /// 服务器描述，对服务器的简短描述
    #[schema(example = "一个有趣的生存服务器")]
    pub desc: String,
    /// 服务器链接，指向服务器详情的链接
    #[schema(example = "https://example.com")]
    pub link: String,
    /// 是否为成员服务器，是否是成员专属服务器
    #[schema(example = true)]
    pub is_member: bool,
    /// 认证模式，服务器使用的认证模式
    #[schema(example = "OFFICIAL")]
    pub auth_mode: ApiAuthMode,
    /// 是否隐藏，服务器是否处于隐藏状态
    #[schema(example = false)]
    pub is_hide: bool,
    /// 服务器标签，与服务器相关的标签
    #[schema(example = json!(["生存", "PVP"]))]
    pub tags: Option<Vec<String>>,
}

/// 搜索响应
#[derive(Serialize, Debug, Deserialize, ToSchema)]
pub struct SearchResponse {
    pub hits: Vec<ServerResult>,
    #[schema(example = 1)]
    pub total: usize,
    #[schema(example = 10)]
    pub limit: usize,
    #[schema(example = 0)]
    pub offset: usize,
    #[schema(example = 12)]
    pub processing_time_ms: u128,
}
