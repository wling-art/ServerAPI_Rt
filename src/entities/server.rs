use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "auth_mode_enum")]
pub enum AuthModeEnum {
    #[sea_orm(string_value = "OFFLINE")]
    Offline,
    #[sea_orm(string_value = "YGGDRASIL")]
    Yggdrasil,
    #[sea_orm(string_value = "OFFICIAL")]
    Official,
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "server_type_enum")]
pub enum ServerTypeEnum {
    #[sea_orm(string_value = "JAVA")]
    Java,
    #[sea_orm(string_value = "BEDROCK")]
    Bedrock,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "server")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    #[sea_orm(column_name = "type")]
    pub server_type: String, // 数据库中是 varchar(50)，不是枚举
    pub version: String,
    #[sea_orm(column_type = "Text")]
    pub desc: String,
    pub link: String,
    pub ip: String,
    pub is_member: bool,
    pub is_hide: bool,
    pub auth_mode: String, // 数据库中是 varchar(50)，不是枚举
    #[sea_orm(column_type = "Text")]
    pub tags: String, // 数据库中是 longtext，不是 JSON
    #[sea_orm(column_name = "cover_hash_id")]
    pub cover_hash: Option<String>,
    pub gallery_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::file::Entity",
        from = "Column::CoverHash",
        to = "super::file::Column::HashValue"
    )]
    CoverFile,
    #[sea_orm(
        belongs_to = "super::gallery::Entity",
        from = "Column::GalleryId",
        to = "super::gallery::Column::Id"
    )]
    Gallery,
    #[sea_orm(has_many = "super::ticket::Entity")]
    Tickets,
    #[sea_orm(has_many = "super::server_status::Entity")]
    ServerStatuses,
    #[sea_orm(has_many = "super::user_server::Entity")]
    UserServers,
}

impl Related<super::file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CoverFile.def()
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Gallery.def()
    }
}

impl Related<super::ticket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tickets.def()
    }
}

impl Related<super::server_status::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServerStatuses.def()
    }
}

impl Related<super::user_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserServers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
