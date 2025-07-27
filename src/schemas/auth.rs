use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthToken {
    /// 服务器列表
    #[schema(example = "fasdfashfksdf")]
    pub access_token: String,
    /// 过期时间
    #[schema(example = 3600)]
    pub expires_in: u64,
}
