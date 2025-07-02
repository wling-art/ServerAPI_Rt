use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use utoipa::{ ToSchema };

// API 层枚举，数据库中存储的是字符串
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ApiServerType {
    #[serde(rename = "JAVA")]
    Java,
    #[serde(rename = "BEDROCK")]
    Bedrock,
}

impl ApiServerType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "JAVA" => Some(Self::Java),
            "BEDROCK" => Some(Self::Bedrock),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Java => "JAVA".to_string(),
            Self::Bedrock => "BEDROCK".to_string(),
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

impl ApiAuthMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "OFFICIAL" => Some(Self::Official),
            "OFFLINE" => Some(Self::Offline),
            "YGGDRASIL" => Some(Self::Yggdrasil),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Official => "OFFICIAL".to_string(),
            Self::Offline => "OFFLINE".to_string(),
            Self::Yggdrasil => "YGGDRASIL".to_string(),
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

/// 创建一个用于文档示例的 ServerListResponse 实例
pub fn create_example_server_list_response() -> ServerListResponse {
    ServerListResponse {
        data: vec![create_example_server_detail()],
        total: 100,
        total_pages: 10,
    }
}

/// 服务器详细信息
///
/// 包含服务器完整信息的结构体，用于API响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerDetail {
    /// 服务器 ID，服务器的唯一标识符
    pub id: i32,
    /// 服务器名称，服务器的名称
    pub name: String,
    /// 服务器 IP，服务器的 IP 地址，若隐藏则为 None
    pub ip: Option<String>,
    /// 服务器类型，服务器所属的类型
    pub r#type: ApiServerType,
    /// 服务器版本，服务器运行的版本
    pub version: String,
    /// 服务器描述，对服务器的简短描述
    pub desc: String,
    /// 服务器链接，指向服务器详情的链接
    pub link: String,
    /// 是否为成员服务器，是否是成员专属服务器
    pub is_member: bool,
    /// 认证模式，服务器使用的认证模式
    pub auth_mode: ApiAuthMode,
    /// 是否隐藏，服务器是否处于隐藏状态
    pub is_hide: bool,
    /// 服务器标签，与服务器相关的标签
    pub tags: Option<Vec<String>>,
    /// 服务器状态，显示服务器的在线状态信息
    pub status: Option<ServerStatus>,
    /// 服务器权限，服务器的权限
    pub permission: String,
    /// 服务器封面，服务器的封面图片链接
    pub cover_url: Option<String>,
}

/// 创建一个用于文档示例的 ServerDetail 实例
pub fn create_example_server_detail() -> ServerDetail {
    let mut players = HashMap::new();
    players.insert("online".to_string(), 10);
    players.insert("max".to_string(), 100);

    ServerDetail {
        id: 1,
        name: "我的世界服务器".to_string(),
        ip: Some("mc.example.com:25565".to_string()),
        r#type: ApiServerType::Java,
        version: "1.20.1".to_string(),
        desc: "一个有趣的生存服务器".to_string(),
        link: "https://example.com".to_string(),
        is_member: true,
        auth_mode: ApiAuthMode::Official,
        is_hide: false,
        tags: Some(vec!["生存".to_string(), "PVP".to_string()]),
        status: Some(create_example_server_status()),
        permission: "guest".to_string(),
        cover_url: Some("https://{我是 CDN 网站}/static/covers/server1.jpg".to_string()),
    }
}

/// 服务器状态信息
///
/// 包含服务器实时状态的结构体，如在线玩家数、延迟等
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerStatus {
    /// 玩家数，当前在线的玩家数量以及最大可容纳的玩家数量
    pub players: HashMap<String, i64>,
    /// 延迟，服务器的延迟时间
    pub delay: f64,
    /// 版本，服务器的软件版本
    pub version: String,
    /// MOTD，服务器的 MOTD 信息
    pub motd: Motd,
    /// 服务器图标，服务器的图标，若无则为 None
    pub icon: Option<String>,
}

/// 创建一个用于文档示例的 ServerStatus 实例
pub fn create_example_server_status() -> ServerStatus {
    let mut players = HashMap::new();
    players.insert("online".to_string(), 10);
    players.insert("max".to_string(), 100);

    ServerStatus {
        players,
        delay: 50.5,
        version: "Paper 1.20.1".to_string(),
        motd: create_example_motd(),
        icon: Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...".to_string()),
    }
}

/// 服务器MOTD信息
///
/// 包含不同格式的服务器消息的结构体
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct Motd {
    /// 纯文本 MOTD，显示服务器的纯文本 MOTD
    pub plain: String,
    /// HTML 格式 MOTD，显示服务器的 HTML 格式 MOTD
    pub html: String,
    /// Minecraft 格式 MOTD，显示 Minecraft 格式的 MOTD
    pub minecraft: String,
    /// ANSI 格式 MOTD，显示 ANSI 格式的 MOTD
    pub ansi: String,
}

/// 创建一个用于文档示例的 Motd 实例
pub fn create_example_motd() -> Motd {
    Motd {
        plain: "欢迎来到我的世界服务器".to_string(),
        html: "<span style='color: green;'>欢迎来到我的世界服务器</span>".to_string(),
        minecraft: "§a欢迎来到我的世界服务器".to_string(),
        ansi: "\\u001b[32m欢迎来到我的世界服务器\\u001b[0m".to_string(),
    }
}
