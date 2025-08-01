use axum_typed_multipart::{FieldData, TryFromMultipart};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use utoipa::ToSchema;
use validator::Validate;

/// API 层枚举，数据库中存储的是字符串
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ApiServerType {
    #[serde(rename = "JAVA")]
    Java,
    #[serde(rename = "BEDROCK")]
    Bedrock,
}

impl FromStr for ApiServerType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "JAVA" => Ok(Self::Java),
            "BEDROCK" => Ok(Self::Bedrock),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ApiAuthMode {
    #[serde(rename = "OFFICIAL")]
    Official,
    #[serde(rename = "OFFLINE")]
    Offline,
    #[serde(rename = "YGGDRASIL")]
    Yggdrasil,
}

impl FromStr for ApiAuthMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OFFICIAL" => Ok(Self::Official),
            "OFFLINE" => Ok(Self::Offline),
            "YGGDRASIL" => Ok(Self::Yggdrasil),
            _ => Err(()),
        }
    }
}

/// 服务器列表响应
///
/// 包含服务器列表和相关统计信息的响应结构体
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerListResponse {
    /// 服务器列表，显示所有的服务器列表
    pub data: Vec<ServerDetail>,
    /// 服务器总数，过滤条件下的所有服务器总数
    #[schema(example = 100)]
    pub total: i64,
    /// 总页数，根据每页数量计算的总页数
    #[schema(example = 10)]
    pub total_pages: i64,
}

/// 服务器详细信息
///
/// 包含服务器完整信息的结构体，用于API响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerDetail {
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
    /// 服务器状态，显示服务器的在线状态信息
    #[schema(example = json!(null))]
    pub stats: Option<ServerStats>,
    /// 服务器权限，服务器的权限
    #[schema(example = "guest")]
    pub permission: String,
    /// 服务器封面，服务器的封面图片链接
    #[schema(example = "https://cdn.example.com/static/covers/server1.jpg")]
    pub cover_url: Option<String>,
}

/// 服务器状态信息
///
/// 包含服务器实时状态的结构体，如在线玩家数、延迟等
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerStats {
    /// 玩家数，当前在线的玩家数量以及最大可容纳的玩家数量
    #[schema(example = json!({"online": 10, "max": 100}))]
    pub players: HashMap<String, i64>,
    /// 延迟，服务器的延迟时间
    #[schema(example = 50.5)]
    pub delay: f64,
    /// 版本，服务器的软件版本
    #[schema(example = "Paper 1.20.1")]
    pub version: String,
    /// MOTD，服务器的 MOTD 信息
    #[schema(
        example = json!({"plain": "欢迎来到我的世界服务器", "html": "<span style='color: green;'>欢迎来到我的世界服务器</span>", "minecraft": "§a欢迎来到我的世界服务器", "ansi": "\\u001b[32m欢迎来到我的世界服务器\\u001b[0m"})
    )]
    pub motd: Motd,
    /// 服务器图标，服务器的图标，若无则为 None
    #[schema(example = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...")]
    pub icon: Option<String>,
}

/// 服务器MOTD信息
///
/// 包含不同格式的服务器消息的结构体
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct Motd {
    /// 纯文本 MOTD，显示服务器的纯文本 MOTD
    #[schema(example = "欢迎来到我的世界服务器")]
    pub plain: String,
    /// HTML 格式 MOTD，显示服务器的 HTML 格式 MOTD
    #[schema(example = "<span style='color: green;'>欢迎来到我的世界服务器</span>")]
    pub html: String,
    /// Minecraft 格式 MOTD，显示 Minecraft 格式的 MOTD
    #[schema(example = "§a欢迎来到我的世界服务器")]
    pub minecraft: String,
    /// ANSI 格式 MOTD，显示 ANSI 格式的 MOTD
    #[schema(example = "\\u001b[32m欢迎来到我的世界服务器\\u001b[0m")]
    pub ansi: String,
}

/// 更新服务器请求
///
/// 用于更新服务器信息的请求结构体
#[derive(Debug, TryFromMultipart, Validate, ToSchema)]
pub struct UpdateServerRequest {
    /// 服务器名称
    #[schema(example = "我的世界服务器")]
    #[validate(length(min = 1, max = 50, message = "服务器名称长度必须在1-50个字符之间"))]
    pub name: String,

    /// 服务器 IP 地址
    #[schema(example = "mc.example.com:25565")]
    #[validate(ip(message = "无效的 IP 地址格式"))]
    pub ip: String,

    /// 服务器描述
    #[schema(
        example = "这是一个非常有趣的生存服务器，我们提供了丰富的游戏内容和友好的社区环境。玩家可以在这里体验到最纯粹的Minecraft生存乐趣。"
    )]
    #[validate(length(min = 100, message = "简介必须大于 100 字"))]
    pub desc: String,

    /// 服务器标签
    #[schema(example = json!(["生存", "PVP"]))]
    #[validate(length(max = 7, message = "tags 数量不能超过 7 个"))]
    pub tags: Vec<String>,

    /// 服务器版本
    #[schema(example = "1.20.1")]
    #[validate(length(min = 1, max = 20, message = "服务器版本长度必须在1-20个字符之间"))]
    pub version: String,

    /// 服务器链接
    #[schema(example = "https://example.com")]
    #[validate(url(message = "无效的链接格式"))]
    pub link: String,

    /// 服务器封面文件
    #[schema(value_type = String, format = Binary)]
    pub cover: Option<FieldData<axum::body::Bytes>>,
}
/// 服务器管理员角色
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ServerManagerRole {
    /// 服主
    #[serde(rename = "owner")]
    Owner,
    /// 管理员
    #[serde(rename = "admin")]
    Admin,
}

/// 管理员信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ManagerInfo {
    /// 用户ID
    pub id: i32,
    /// 显示名称
    pub display_name: String,
    /// 是否活跃
    pub is_active: bool,
    /// 头像URL
    pub avatar_url: String,
}

/// 服务器管理员列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerManagersResponse {
    /// 服主列表
    pub owners: Vec<ManagerInfo>,
    /// 管理员列表
    pub admins: Vec<ManagerInfo>,
}

/// 单个相册图片信息
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GalleryImage {
    /// 图片ID
    #[schema(example = 10)]
    pub id: i32,

    /// 图片标题
    #[schema(example = "建筑全景")]
    pub title: String,

    /// 图片描述
    #[schema(example = "主城鸟瞰图")]
    pub description: String,

    /// 图片URL地址
    #[schema(example = "https://cdn.example.com/gallery1.png")]
    pub image_url: String,
}

/// 服务器相册响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerGallery {
    /// 服务器ID
    #[schema(example = 2)]
    pub id: i32,

    /// 服务器名称
    #[schema(example = "服务器名称")]
    pub name: String,

    /// 相册图片列表
    pub gallery_images: Vec<GalleryImage>,
}

/// 添加画册图片的请求结构体（用于OpenAPI文档）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GalleryImageRequest {
    /// 图片标题
    #[schema(example = "主城建筑")]
    pub title: String,

    /// 图片描述
    #[schema(example = "这是一个非常棒的主城建筑，展示了我们服务器的建筑水平")]
    pub description: String,

    /// 图片文件
    #[schema(value_type = String, format = Binary)]
    pub image: String,
}

/// 添加画册图片的请求结构体
#[derive(Debug, TryFromMultipart, Validate, ToSchema)]
pub struct GalleryImageSchema {
    /// 图片标题
    #[schema(example = "主城建筑")]
    #[validate(length(min = 1, max = 100, message = "标题长度必须在1-100个字符之间"))]
    pub title: String,

    /// 图片描述
    #[schema(example = "这是一个非常棒的主城建筑，展示了我们服务器的建筑水平")]
    #[validate(length(min = 1, max = 500, message = "描述长度必须在1-500个字符之间"))]
    pub description: String,

    /// 图片文件
    #[schema(value_type = String, format = Binary)]
    pub image: FieldData<axum::body::Bytes>,
}

/// 通用成功响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
    /// 成功消息
    #[schema(example = "操作成功")]
    pub message: String,
}

/// 服务器总玩家数响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerTotalPlayers {
    /// 服务器总玩家数
    #[schema(example = 1234)]
    pub total_players: i32,
}
